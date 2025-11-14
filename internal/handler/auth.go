package handler

import (
  "log/slog"
  "net/http"
  "time"

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

  claims := CustomClaims{FirstName: user.FirstName}
  token, err := claims.GenerateToken()
  if err != nil {
    slog.Error("token generation", "error", err)
    return ctx.NoContent(http.StatusInternalServerError)
  }

  cookie := http.Cookie{
    Name:     "jwt",
    Value:    token,
    HttpOnly: true,
    Secure:   true,
    Path:     "/",
    Expires:  time.Now().Add(lifespan),
  }

  ctx.SetCookie(&cookie)
  slog.Info("logged in", "name", user.FirstName)

  return ctx.NoContent(http.StatusOK)
}

func (h *AuthHandler) Protected(ctx echo.Context) error {
  claims, err := DecodeToken(ctx)
  if err != nil {
    slog.Error("token decoding", "error", err)
    return ctx.NoContent(http.StatusInternalServerError)
  }

  slog.Info("decoded token", "name", claims.FirstName)
  return ctx.String(http.StatusOK, claims.FirstName)
}
