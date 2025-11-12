package handler

import (
  "github.com/brooknullsh/railway-playground/internal/store"
  echotok "github.com/labstack/echo-jwt/v4"
  "github.com/labstack/echo/v4"
)

type Handlers struct {
  index *IndexHandler
  auth  *AuthHandler
}

func NewWithState(store *store.Store) *Handlers {
  return &Handlers{index: &IndexHandler{store}, auth: &AuthHandler{store}}
}

func (h *Handlers) RegisterRoutes(app *echo.Echo) {
  app.GET("/", h.index.Root)

  authGroup := app.Group("/auth")
  authGroup.POST("/login", h.auth.Login)

  authMiddleware := echotok.JWT([]byte("SECRET"))
  authGroup.GET("", h.auth.Test, authMiddleware)
}
