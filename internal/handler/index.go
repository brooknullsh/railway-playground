package handler

import (
  "net/http"

  "github.com/brooknullsh/railway-playground/internal/store"
  "github.com/labstack/echo/v4"
)

type IndexHandler struct {
  store *store.Store
}

func (h *IndexHandler) Root(ctx echo.Context) error {
  return ctx.String(http.StatusOK, "Hello, world!")
}
