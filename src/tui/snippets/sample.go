// Syntax preview: comments, keywords, types, strings, escapes, labels.
package main

import (
    "fmt"
    "regexp"
)

const maxItems = 100
const version = "1.0.0"

type Config struct {
    Name    string
    Count   int
    Enabled bool
}

func NewConfig(name string) *Config {
    return &Config{Name: name, Count: 0, Enabled: true}
}

func process(items []int) map[int]bool {
    result := make(map[int]bool)
outer:
    for _, item := range items {
        if item < 0 {
            continue outer
        }
        result[item] = item%2 == 0
    }
    return result
}

func main() {
    msg := "Hello\tWorld\n"
    pattern := regexp.MustCompile(`\d+`)
    config := NewConfig("example")
    fmt.Printf("Config: %+v, msg: %q, pattern: %v\n", config, msg, pattern)
    fmt.Printf("Result: %v\n", process([]int{1, 2, -3, 4, 5}))
}
