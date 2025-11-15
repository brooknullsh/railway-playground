package handler

import (
  "log/slog"
  "net/http"
  "time"

  "github.com/brooknullsh/railway-playground/internal/middleware"
  "github.com/brooknullsh/railway-playground/internal/store"
  "github.com/labstack/echo/v4"
)

type AuthHandler struct {
  store *store.Store
}

type LoginRequest struct {
  FirstName string `json:"firstName" validate:"required"`
}

func (h *AuthHandler) Login(ctx echo.Context) error {
  request := ctx.Request().Context()

  var body LoginRequest
  if err := ctx.Bind(&body); err != nil {
    slog.Error("invalid request body", "error", err)
    return ctx.NoContent(http.StatusBadRequest)
  }

  user, err := h.store.GetUserByName(request, body.FirstName)
  if err != nil {
    slog.Error("user query", "error", err)
    return ctx.NoContent(http.StatusInternalServerError)
  }

  claims := middleware.CustomClaims{FirstName: user.FirstName}
  token, err := claims.GenerateJWT()
  if err != nil {
    slog.Error("token generation", "error", err)
    return ctx.NoContent(http.StatusInternalServerError)
  }

  cookie := claims.BuildCookie(token)
  ctx.SetCookie(cookie)

  slog.Info("logged in", "exp", claims.ExpiresAt.Time)
  return ctx.NoContent(http.StatusOK)
}

func (h *AuthHandler) Protected(ctx echo.Context) error {
  claims := ctx.Get("claims").(*middleware.CustomClaims)
  slog.Info("decoded token", "exp", claims.ExpiresAt.Time)

  return ctx.String(http.StatusOK, claims.FirstName)
}

func (h *AuthHandler) ProtectedMiddleware(next echo.HandlerFunc, ctx echo.Context) error {
  claims, err := middleware.DecodeJWTFromMiddleware(ctx)
  if err != nil {
    slog.Error("token decoding", "error", err)
    return ctx.NoContent(http.StatusUnauthorized)
  }

  if time.Until(claims.ExpiresAt.Time) < time.Minute {
    cookie, code := claims.RefreshCookie()
    if code != http.StatusOK {
      return ctx.NoContent(code)
    }

    slog.Info("token refreshed", "exp", claims.ExpiresAt.Time)
    ctx.SetCookie(cookie)
  }

  ctx.Set("claims", claims)
  return next(ctx)
}
