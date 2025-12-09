package handler

import (
  "bytes"
  "encoding/json"
  "net/http"
  "testing"

  "github.com/brooknullsh/railway-playground/internal/handler"
  "github.com/stretchr/testify/assert"
)

func TestIndex(test *testing.T) {
  res, err := http.Get("http://localhost:8080")

  assert.NoError(test, err)
  assert.Equal(test, http.StatusUnauthorized, res.StatusCode)
}

func TestLogin(test *testing.T) {
  var body handler.LoginBody
  body.Id = 1

  json, err := json.Marshal(&body)
  assert.NoError(test, err)

  res, err := http.Post("http://localhost:8080/login", "application/json", bytes.NewBuffer(json))

  assert.NoError(test, err)
  assert.Equal(test, http.StatusOK, res.StatusCode)
}
