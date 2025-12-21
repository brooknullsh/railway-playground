package test

import (
  "fmt"
  "strings"
  "testing"
)

func AssertEqual[T comparable](test *testing.T, actual, expected T) {
  test.Helper()

  if actual != expected {
    failLog("%v != %v", actual, expected)
    test.FailNow()
  }
}

func AssertNoErr(test *testing.T, actual any) {
  test.Helper()

  if actual != nil {
    failLog("%v != nil", actual)
    test.FailNow()
  }
}

func failLog(format string, args ...any) {
  var builder strings.Builder
  builder.WriteString("\033[0;31mERROR\033[0m ")
  builder.WriteString(format)

  fmt.Printf(builder.String(), args...)
}
