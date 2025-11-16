package handler

import (
  "log/slog"
  "net/http"

  "github.com/brooknullsh/railway-playground/internal/store"
  "github.com/labstack/echo/v4"
  echomiddleware "github.com/labstack/echo/v4/middleware"
)

type Handler struct {
  index *IndexHandler
  auth  *AuthHandler
}

func InitialiseWithState(app *echo.Echo, store *store.Store) {
  handlers := &Handler{&IndexHandler{store}, &AuthHandler{store}}
  handlers.registerRoutes(app)
}

func (h *Handler) registerRoutes(app *echo.Echo) {
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

  protected := func(next echo.HandlerFunc) echo.HandlerFunc {
    return func(ctx echo.Context) error {
      return h.auth.ProtectedMiddleware(next, ctx)
    }
  }

  app.GET("/", h.index.Root, protected)
  app.POST("/login", h.auth.Login)
}

func requestLogger(_ echo.Context, data echomiddleware.RequestLoggerValues) error {
  if data.Status >= http.StatusBadRequest {
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
