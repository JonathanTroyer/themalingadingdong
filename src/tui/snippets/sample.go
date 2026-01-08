// Example Go code for syntax highlighting preview.
package main

import (
    "errors"
    "fmt"
)

const maxRetries = 3
const apiURL = "https://api.example.com"

type Config struct {
    Name    string
    Enabled bool
    Retries int
}

func NewConfig(name string) *Config {
    return &Config{
        Name:    name,
        Enabled: true,
        Retries: maxRetries,
    }
}

func (c *Config) Validate() error {
    if c.Name == "" {
        return errors.New("name cannot be empty")
    }
    if c.Retries > 10 {
        return fmt.Errorf("retries %d exceeds maximum", c.Retries)
    }
    return nil
}

func processItems(items []int) map[int]bool {
    result := make(map[int]bool)
    for _, item := range items {
        isEven := item%2 == 0
        result[item] = isEven
    }
    return result
}

func parseEmail(text string) (string, bool) {
    for i, ch := range text {
        if ch == '@' && i > 0 && i < len(text)-1 {
            return text, true
        }
    }
    return "", false
}

func main() {
    config := NewConfig("example")
    if err := config.Validate(); err != nil {
        fmt.Printf("Error: %v\n", err)
        return
    }
    fmt.Printf("Config: %+v\n", config)

    items := []int{1, 2, 3, 4, 5}
    processed := processItems(items)
    fmt.Printf("Processed: %v\n", processed)
}
