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
  reqCtx := ctx.Request().Context()

  var request LoginRequest
  if err := ctx.Bind(&request); err != nil {
    return WarnAndRespond(ctx, "[LOGIN][BINDING]", err, http.StatusBadRequest)
  }

  user, err := h.store.GetUserByName(reqCtx, request.FirstName)
  if err != nil {
    return WarnAndRespond(ctx, "", err, http.StatusBadRequest)
  }

  claims := CustomClaims{FirstName: user.FirstName}

  token, err := claims.GenerateToken()
  if err != nil {
    return WarnAndRespond(ctx, "", err, http.StatusInternalServerError)
  }

  tokenCookie := http.Cookie{
    Name:     "jwt",
    Value:    token,
    HttpOnly: true,
    Secure:   true,
    Path:     "/",
    Expires:  time.Now().Add(jwtDuration),
  }

  ctx.SetCookie(&tokenCookie)
  slog.Info("logged in", "name", user.FirstName)

  // TODO: May just need token stored in HTTP-only cookie
  return ctx.JSON(http.StatusOK, map[string]string{"token": token})
}

func (h *AuthHandler) Protected(ctx echo.Context) error {
  claims, err := DecodeToken(ctx)
  if err != nil {
    return WarnAndRespond(ctx, "", err, http.StatusUnauthorized)
  }

  slog.Info("decoded JWT", "name", claims.FirstName)
  return ctx.String(http.StatusOK, claims.FirstName)
}
