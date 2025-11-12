package handler

import (
  "log/slog"

  "github.com/brooknullsh/railway-playground/internal/store"
  echojwt "github.com/labstack/echo-jwt/v4"
  "github.com/labstack/echo/v4"
)

type Handlers struct {
  index *IndexHandler
  auth  *AuthHandler
}

func (h *Handlers) RegisterRoutes(app *echo.Echo) {
  secret := SecretKeyBytes()
  authMiddleware := echojwt.JWT(secret)

  app.GET("/", h.index.Root)

  authGroup := app.Group("/auth")
  authGroup.POST("/login", h.auth.Login)
  authGroup.GET("/protected", h.auth.Protected, authMiddleware)
}

func NewWithState(store *store.Store) *Handlers {
  return &Handlers{index: &IndexHandler{store}, auth: &AuthHandler{store}}
}

func WarnAndRespond(ctx echo.Context, prefix string, err error, status int) error {
  errorMsg := err.Error()
  // The prefix can be empty if the error is internal, meaning it already has a
  // prefix. Otherwise, useful for context.
  //
  // TODO: Can this be replaced with a built-in middleware logger?
  slog.Warn(prefix + " " + errorMsg)

  return ctx.NoContent(status)
}
