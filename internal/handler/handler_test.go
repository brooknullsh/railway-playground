package handler

import (
  "io"
  "net/http"
  "testing"

  "github.com/stretchr/testify/assert"
)

func TestIndex(test *testing.T) {
  res, err := http.Get("http://localhost:8080")

  assert.Nil(test, err)
  assert.Equal(test, http.StatusOK, res.StatusCode)

  defer res.Body.Close()
  body, err := io.ReadAll(res.Body)

  assert.Nil(test, err)
  assert.Equal(test, "Alice", string(body))
}
