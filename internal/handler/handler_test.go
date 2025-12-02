package handler

import (
  "io"
  "net/http"
  "testing"

  "github.com/gofiber/utils"
)

func TestIndex(test *testing.T) {
  res, err := http.Get("http://localhost:8080")

  utils.AssertEqual(test, nil, err, "request failed")
  utils.AssertEqual(test, http.StatusOK, res.StatusCode, "unexpected status code")

  defer res.Body.Close()
  body, err := io.ReadAll(res.Body)

  utils.AssertEqual(test, nil, err, "response body failed")
  utils.AssertEqual(test, "Alice", string(body), "unexpected response body")
}
