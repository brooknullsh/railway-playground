package handler

import (
  "log/slog"
  "net/http"

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
  reqCtx := ctx.Request().Context()

  var request LoginRequest
  if err := ctx.Bind(&request); err != nil {
    slog.Error("[BINDING] " + err.Error())
    return ctx.NoContent(http.StatusBadRequest)
  }

  user, err := h.store.GetUserByName(reqCtx, request.FirstName)
  if err != nil {
    slog.Warn("[LOGIN] " + err.Error())
    return ctx.NoContent(http.StatusBadRequest)
  }

  claims := CustomClaims{FirstName: user.FirstName}

  token, err := claims.GenerateToken()
  if err != nil {
    slog.Error(err.Error())
    return ctx.NoContent(http.StatusInternalServerError)
  }

  slog.Info("authenticated", "user", user.FirstName)
  return ctx.JSON(http.StatusOK, map[string]string{"token": token})
}

func (h *AuthHandler) Test(ctx echo.Context) error {
  firstName, err := DecodeToken(ctx)
  if err != nil {
    slog.Error(err.Error())
    return ctx.NoContent(http.StatusUnauthorized)
  }

  return ctx.String(http.StatusOK, firstName.FirstName)
}
