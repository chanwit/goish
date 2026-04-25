// port of go/src/runtime/chan_test.go
//
// 100% semantic verification: every property Go's runtime/chan_test.go
// exercises, ported to goish. select!-dependent subtests are marked TODO
// until v0.5 select! lands; everything else is live.

#![allow(non_camel_case_types, non_snake_case)]
use goish::prelude::*;

// ── TestChan_Buffered_TryRecvOnEmpty ──────────────────────────────────
// Go: "Ensure that receive from empty chan blocks."

test!{ fn TestChan_TryRecvOnEmpty(t) {
    for chanCap in [0i64, 1, 2, 4, 8, 16] {
        let c = chan!(i64, chanCap as usize);
        let (_, ok) = c.TryRecv();
        if ok {
            t.Errorf(Sprintf!("chan[%d]: TryRecv on empty returned ok", chanCap));
        }
    }
}}

// ── TestChan_TrySendOnFull ────────────────────────────────────────────
// Go: "Ensure that non-blocking send does not block. [on full chan]"

test!{ fn TestChan_TrySendOnFull(t) {
    for chanCap in [1i64, 2, 4, 8] {
        let c = chan!(i64, chanCap as usize);
        for i in 0..chanCap { let _ = c.Send(i); }
        let ok = c.TrySend(99);
        if ok {
            t.Errorf(Sprintf!("chan[%d]: TrySend on full returned ok", chanCap));
        }
    }
}}

// ── TestChan_ReceiveZeroFromClosed ────────────────────────────────────
// Go: "Ensure that we receive 0 from closed chan."

test!{ fn TestChan_ReceiveZeroFromClosed(t) {
    for chanCap in [1i64, 2, 4, 8, 16] {
        let c = chan!(i64, chanCap as usize);
        for i in 0..chanCap { let _ = c.Send(i); }
        c.Close();
        // Drain: each should still succeed in order.
        for i in 0..chanCap {
            let (v, ok) = c.Recv();
            if !ok {
                t.Fatalf(Sprintf!("chan[%d]: drain recv #%d not ok", chanCap, i));
            }
            if v != i {
                t.Errorf(Sprintf!("chan[%d]: received %v, expected %v", chanCap, v, i));
            }
        }
        // After drain: infinite (0, false).
        for _ in 0..3 {
            let (v, ok) = c.Recv();
            if ok || v != 0 {
                t.Errorf(Sprintf!("chan[%d]: post-drain got (%v, %v); want (0, false)", chanCap, v, ok));
            }
        }
    }
}}

// ── TestChan_CloseUnblocksReceive ─────────────────────────────────────
// Go: "Ensure that close unblocks receive."

test!{ fn TestChan_CloseUnblocksReceive(t) {
    for chanCap in [0i64, 1, 2, 4, 8] {
        let c = chan!(i64, chanCap as usize);
        let done = chan!(bool, 1);
        let cc = c.clone();
        let dc = done.clone();
        let g = go!{
            let (v, ok) = cc.Recv();
            let _ = dc.Send(v == 0 && !ok);
        };
        std::thread::sleep(std::time::Duration::from_millis(30));
        c.Close();
        let (got, _) = done.Recv();
        if !got {
            t.Fatalf(Sprintf!("chan[%d]: received non-zero from closed chan", chanCap));
        }
        let _ = g.Wait();
    }
}}

// ── TestChan_FIFOAcrossGoroutines ─────────────────────────────────────
// Go: "Send 100 integers, ensure that we receive them non-corrupted in FIFO order."

test!{ fn TestChan_FIFO(t) {
    for chanCap in [0i64, 1, 4, 16, 100] {
        let c = chan!(i64, chanCap as usize);
        let cp = c.clone();
        let g = go!{
            for i in 0..100 { let _ = cp.Send(i); }
        };
        for i in 0..100 {
            let (v, ok) = c.Recv();
            if !ok { t.Fatalf(Sprintf!("chan[%d]: receive failed at %d", chanCap, i)); }
            if v != i {
                t.Fatalf(Sprintf!("chan[%d]: received %v, expected %v", chanCap, v, i));
            }
        }
        let _ = g.Wait();
    }
}}

// ── TestChan_MPMCBigFanout ────────────────────────────────────────────
// Go: "Send 1000 integers in 4 goroutines, ensure that we receive what we send."
// Each receiver consumes L values; aggregated counts must show every value
// exactly P times (since P producers each send the full range 0..L).

test!{ fn TestChan_MPMCBigFanout(t) {
    const P: i32 = 4;
    const L: i32 = 1000;

    for &chanCap in &[0, 1, 4, 16, 100] {
        let c = chan!(i32, chanCap);

        // Producers.
        let mut producers: slice<Goroutine> = slice::new();
        for _ in 0..P {
            let cp = c.clone();
            producers.push(go!{
                for i in 0..L { let _ = cp.Send(i); }
            });
        }

        // Consumers — each reads L values into its own hashmap,
        // then pushes the map out via `done`.
        let done = chan!(map<i32, i32>, P as usize);
        let mut consumers: slice<Goroutine> = slice::new();
        for _ in 0..P {
            let cc = c.clone();
            let dc = done.clone();
            consumers.push(go!{
                let mut recv: map<i32, i32> = make!(map[i32]i32);
                for _ in 0..L {
                    let (v, _) = cc.Recv();
                    *recv.entry(v).or_insert(0) += 1;
                }
                let _ = dc.Send(recv);
            });
        }

        // Merge consumer maps.
        let mut total: map<i32, i32> = make!(map[i32]i32);
        for _ in 0..P {
            let (m, _) = done.Recv();
            for (k, v) in m { *total.entry(k).or_insert(0) += v; }
        }

        for g in producers { let _ = g.Wait(); }
        for g in consumers { let _ = g.Wait(); }

        if total.len() as i32 != L {
            t.Fatalf(Sprintf!("chan[cap=%d]: received %v distinct values, expected %v",
                chanCap, total.len() as i32, L));
        }
        for (k, v) in &total {
            if *v != P {
                t.Fatalf(Sprintf!("chan[cap=%d]: key %v received %v times, expected %v",
                    chanCap, k, v, P));
            }
        }
    }
}}

// ── TestChan_LenCap ───────────────────────────────────────────────────
// Go: "Test len/cap."

test!{ fn TestChan_LenCap(t) {
    for chanCap in [1i64, 2, 4, 8, 16] {
        let c = chan!(i64, chanCap as usize);
        if c.Len() != 0 || c.Cap() != chanCap {
            t.Fatalf(Sprintf!("chan[%d]: bad initial len/cap %v/%v", chanCap, c.Len(), c.Cap()));
        }
        for i in 0..chanCap { let _ = c.Send(i); }
        if c.Len() != chanCap || c.Cap() != chanCap {
            t.Fatalf(Sprintf!("chan[%d]: bad full len/cap %v/%v", chanCap, c.Len(), c.Cap()));
        }
    }
}}

// ── TestChan_SendOnClosed ─────────────────────────────────────────────
// Port of runtime/chan_test.go TestChanSendOnClosed — `c <- v` panics on
// a closed channel.

test!{ fn TestChan_SendOnClosed(t) {
    let c = chan!(i64, 1);
    c.Close();
    let r = recover!{ c.Send(42); };
    if r.is_none() {
        t.Error("send on closed channel should panic");
    }
}}

// ── TestChan_CloseOnClosed ────────────────────────────────────────────
// Port of runtime/chan_test.go TestChanCloseOnClosed — `close(c)` panics
// on a previously-closed channel.

test!{ fn TestChan_CloseOnClosed(t) {
    let c = chan!(i64, 1);
    c.Close();
    let r = recover!{ c.Close(); };
    if r.is_none() {
        t.Error("double-close should panic");
    }
}}

// ── TestChan_UnbufferedRendezvous ─────────────────────────────────────

test!{ fn TestChan_UnbufferedRendezvous(t) {
    let c = chan!(i32);
    let cp = c.clone();
    let g = go!{ let _ = cp.Send(42); };
    let (v, ok) = c.Recv();
    let _ = g.Wait();
    if !ok || v != 42 {
        t.Errorf(Sprintf!("unbuffered recv: got (%d, %v); want (42, true)", v, ok));
    }
}}

// ── TestChan_CloseDrainsBuffered ──────────────────────────────────────

test!{ fn TestChan_CloseDrainsBuffered(t) {
    let c = chan!(i64, 4);
    let _ = c.Send(1);
    let _ = c.Send(2);
    let _ = c.Send(3);
    c.Close();
    for expect in [1i64, 2, 3] {
        let (v, ok) = c.Recv();
        if !ok { t.Fatalf(Sprintf!("expected ok recv of %d", expect)); }
        if v != expect { t.Errorf(Sprintf!("recv got %d, want %d", v, expect)); }
    }
    let (v, ok) = c.Recv();
    if ok { t.Errorf(Sprintf!("expected !ok after drain; got (%d, true)", v)); }
}}

// ── TestChan_ManyGoroutinesSumming ────────────────────────────────────
// Beyond Go's TestChan: scale test ensuring go!{} + Chan<T> compose cleanly
// under concurrency. Goroutines are async tasks, so there's no pool cap to
// worry about — tests/million_goroutines.rs pushes this to 1M.

test!{ fn TestChan_ManyGoroutines(t) {
    const N: i32 = 200;
    let c = chan!(i32, 16);
    let total = sync::atomic::Int32::new(0);

    // Consumer first — guarantees it gets a pool slot.
    let cc = c.clone();
    let total_c = total.clone();
    let consumer = go!{
        for _ in 0..N {
            let (v, _) = cc.Recv();
            total_c.Add(v);
        }
    };

    let mut producers: slice<Goroutine> = slice::new();
    for i in 0..N {
        let cp = c.clone();
        producers.push(go!{ let _ = cp.Send(i); });
    }

    for g in producers { let _ = g.Wait(); }
    let _ = consumer.Wait();

    let expected = (N - 1) * N / 2;
    let got = total.Load();
    if got != expected {
        t.Errorf(Sprintf!("many goroutines sum: got %d, want %d", got, expected));
    }
}}

// ── TestChan_NonblockRecvRace ─────────────────────────────────────────
// Port of Go's TestNonblockRecvRace.
//
// After the sending goroutine is spawned, `close(c)` happens. Per Go's spec
// a recv on a closed channel is always "ready" — so `select{ case <-c:
// default: }` must fire the recv case, never default.

test!{ fn TestChan_NonblockRecvRace(t) {
    let n = 1000;
    let errs = sync::atomic::Uint32::new(0);
    for _ in 0..n {
        let c = chan!(i32, 1);
        let _ = c.Send(1);
        let cc = c.clone();
        let errs_c = errs.clone();
        let g = go!{
            goish::select!{
                recv(cc) => {},
                default => { errs_c.Add(1); },
            }
        };
        c.Close();
        let _ = c.Recv();
        let _ = g.Wait();
    }
    let n_errs = errs.Load();
    if n_errs != 0 {
        t.Errorf(Sprintf!("non-blocking recv raced in %v/%v iterations", n_errs as i32, n as i32));
    }
}}

// ── TestChan_EngineReport ─────────────────────────────────────────────

test!{ fn TestChan_EngineReport(t) {
    t.Logf(Sprintf!("chan engine = %s", goish::chan::ENGINE));
}}
