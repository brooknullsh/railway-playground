package handler

import (
  "log/slog"
  "net/http"

  "github.com/brooknullsh/railway-playground/internal/store"
  echojwt "github.com/labstack/echo-jwt/v4"
  "github.com/labstack/echo/v4"
  echomiddleware "github.com/labstack/echo/v4/middleware"
)

type Handlers struct {
  index *IndexHandler
  auth  *AuthHandler
}

func (h *Handlers) RegisterRoutes(app *echo.Echo) {
  secret := SecretKeyBytes()
  protected := echojwt.JWT(secret)

  logger := echomiddleware.RequestLoggerConfig{
    LogURI:        true,
    LogStatus:     true,
    LogMethod:     true,
    LogRemoteIP:   true,
    LogValuesFunc: requestLogger,
  }

  limiter := echomiddleware.RateLimiterConfig{
    Store: echomiddleware.NewRateLimiterMemoryStore(20),
    IdentifierExtractor: func(ctx echo.Context) (string, error) {
      return ctx.RealIP(), nil
    },
    DenyHandler: func(ctx echo.Context, _ string, _ error) error {
      return ctx.NoContent(http.StatusTooManyRequests)
    },
  }

  app.Use(echomiddleware.RequestLoggerWithConfig(logger))
  app.Use(echomiddleware.RateLimiterWithConfig(limiter))

  app.GET("/", h.index.Root)
  app.POST("/login", h.auth.Login)
  app.GET("/protected", h.auth.Protected, protected)
}

func InitialiseWithState(app *echo.Echo, store *store.Store) {
  handlers := &Handlers{index: &IndexHandler{store}, auth: &AuthHandler{store}}
  handlers.RegisterRoutes(app)
}

func requestLogger(_ echo.Context, data echomiddleware.RequestLoggerValues) error {
  if data.Error != nil {
    slog.Error(
      "failed request",
      "uri", data.URI,
      "error", data.Error,
      "status", data.Status,
      "method", data.Method,
      "remote_ip", data.RemoteIP,
    )
  } else {
    slog.Info(
      "successful request",
      "uri", data.URI,
      "status", data.Status,
      "method", data.Method,
      "remote_ip", data.RemoteIP,
    )
  }

  return nil
}
