package handler

import (
  "log/slog"
  "net/http"

  "github.com/brooknullsh/railway-playground/internal/middleware"
  "github.com/brooknullsh/railway-playground/internal/store"
  "github.com/labstack/echo/v4"
)

type AuthHandler struct {
  store *store.Store
}

func (h *AuthHandler) Login(ctx echo.Context) error {
  type loginRequest struct {
    FirstName string `json:"firstName" validate:"required"`
  }

  var body loginRequest
  if err := ctx.Bind(&body); err != nil {
    slog.Error("invalid request body", "error", err)
    return ctx.NoContent(http.StatusBadRequest)
  }

  // TODO: Move query here
  user, err := h.store.User.GetUserByName(ctx.Request().Context(), body.FirstName)
  if err != nil {
    slog.Error("retrieving user by name", "name", body.FirstName, "error", err)
    return ctx.NoContent(http.StatusInternalServerError)
  }

  newAccessClaims := middleware.CustomClaims{FirstName: user.FirstName}
  accessTokenString, accessErr := newAccessClaims.GenerateJWT(middleware.AccessLifespan)
  newRefreshClaims := middleware.CustomClaims{FirstName: user.FirstName}
  refreshTokenString, refreshErr := newRefreshClaims.GenerateJWT(middleware.RefreshLifespan)

  if accessErr != nil || refreshErr != nil {
    slog.Error("creating access and refresh tokens", "error", err)
    return ctx.NoContent(http.StatusInternalServerError)
  }

  if err := h.store.Auth.SetRefreshToken(ctx, refreshTokenString, user.FirstName); err != nil {
    slog.Error("saving refresh token to user", "user_name", user.FirstName, "error", err)
    return ctx.NoContent(http.StatusInternalServerError)
  }

  ctx.SetCookie(newAccessClaims.BuildCookie(accessTokenString, middleware.AccessName))
  ctx.SetCookie(newRefreshClaims.BuildCookie(refreshTokenString, middleware.RefreshName))
  return ctx.NoContent(http.StatusOK)
}

func (h *AuthHandler) Protected(ctx echo.Context) error {
  claims := ctx.Get("user").(*middleware.CustomClaims)
  return ctx.String(http.StatusOK, claims.FirstName)
}

func (h *AuthHandler) ProtectedMiddleware(next echo.HandlerFunc, ctx echo.Context) error {
  userAuthCtx := middleware.UserAuthContext{
    AccessToken:   "",
    RefreshToken:  "",
    AccessClaims:  &middleware.CustomClaims{},
    RefreshClaims: &middleware.CustomClaims{},
  }

  if err := userAuthCtx.SaveJWTsFromCookies(h.store, ctx); err != nil {
    slog.Error("saving the tokens from request", "error", err)
    return ctx.NoContent(http.StatusUnauthorized)
  }

  ctx.Set("user", userAuthCtx.AccessClaims)
  return next(ctx)
}

// TODO: Move all of below to middleware package
// TODO: Check overlap with ProtectedMiddleware, there's loads
// TODO: Sort out status code returns
// TODO: Move queries to store/auth
