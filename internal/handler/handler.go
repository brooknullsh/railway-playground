package handler

import (
  "github.com/brooknullsh/railway-playground/internal/store"
  "github.com/labstack/echo/v4"
)

type Handlers struct {
  index *IndexHandler
}

func New(store *store.Store) *Handlers {
  return &Handlers{index: &IndexHandler{store}}
}

func (h *Handlers) Register(app *echo.Echo) {
  app.GET("/", h.index.Root)
}
