package test

import (
  "net/http"
  "testing"
)

const INDEX_ROUTE = "http://localhost:8080"

func TestIndex(test *testing.T) {
  req, err := http.Get(INDEX_ROUTE)
  AssertNoErr(test, err)

  AssertEqual(test, req.StatusCode, http.StatusOK)
}
