package main

import (
	"errors"
	"fmt"
	"strconv"
)

type PathTest struct {
	path, result string
}

var ErrNotFound = errors.New("not found")

const (
	Sunday = iota
	Monday
	Tuesday
)

func divide(a, b int64) (int64, error) {
	if b == 0 {
		return 0, errors.New("divide by zero")
	}
	return a / b, nil
}

func main() {
	cases := []PathTest{
		{"a", "A"},
		{"b", "B"},
	}
	for i, c := range cases {
		fmt.Printf("case %d: %s -> %s\n", i, c.path, c.result)
	}

	nums := []int{1, 2, 3, 4, 5}
	total := 0
	for _, n := range nums {
		total += n
	}
	fmt.Println("total:", total)

	m := map[string]int{"a": 1, "b": 2}
	for k, v := range m {
		fmt.Println(k, v)
	}

	q, err := divide(10, 2)
	if err != nil {
		fmt.Println("error:", err)
	} else {
		fmt.Println("q =", q)
	}

	s, err := strconv.Atoi("42")
	if err == nil {
		fmt.Println(s)
	}

	for i := 0; i < 3; i++ {
		fmt.Println(i)
	}
}
