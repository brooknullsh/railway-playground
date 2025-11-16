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

  user, err := h.store.User.GetUserByName(ctx, body.FirstName)
  if err != nil {
    slog.Error("retrieving user", "name", body.FirstName, "error", err)
    return ctx.NoContent(http.StatusInternalServerError)
  }

  claims := middleware.JWTClaims{FirstName: user.FirstName}
  accessToken, accessErr := claims.NewToken(middleware.Access)
  refreshToken, refreshErr := claims.NewToken(middleware.Refresh)

  if accessErr != nil || refreshErr != nil {
    slog.Error("creating new tokens", "error", accessErr, "error", refreshErr)
    return ctx.NoContent(http.StatusInternalServerError)
  }

  if err := h.store.Auth.SetRefreshToken(ctx, refreshToken, user.FirstName); err != nil {
    slog.Error("saving new refresh token", "name", user.FirstName, "error", err)
    return ctx.NoContent(http.StatusInternalServerError)
  }

  accessCookie := claims.BuildCookie(middleware.Access, accessToken)
  refreshCookie := claims.BuildCookie(middleware.Refresh, refreshToken)
  ctx.SetCookie(accessCookie)
  ctx.SetCookie(refreshCookie)

  return ctx.NoContent(http.StatusOK)
}

func (h *AuthHandler) ProtectedMiddleware(next echo.HandlerFunc, ctx echo.Context) error {
  userCtx := middleware.UserContext{
    AccessToken:   "",
    RefreshToken:  "",
    AccessClaims:  &middleware.JWTClaims{},
    RefreshClaims: &middleware.JWTClaims{},
  }

  refreshCookie, err := ctx.Cookie(middleware.RefreshCookieName)
  if err != nil {
    slog.Warn("unauthorised request in protected route")
    return ctx.NoContent(http.StatusUnauthorized)
  }

  userCtx.RefreshToken = refreshCookie.Value
  accessCookie, err := ctx.Cookie(middleware.AccessCookieName)

  var shouldRefreshAccess bool
  if err == nil {
    if err := userCtx.AccessClaims.DecodeTokenIntoClaims(accessCookie.Value, middleware.Access); err != nil {
      slog.Error("decoding access token from request", "error", err)
      return ctx.NoContent(http.StatusUnauthorized)
    }

    shouldRefreshAccess = userCtx.AccessClaims.NeedsRefresh()
  }

  if err != nil || shouldRefreshAccess {
    if exists := h.store.Auth.RefreshTokenExists(ctx, userCtx.RefreshToken); !exists {
      slog.Warn("invalid refresh token with no access token")
      return ctx.NoContent(http.StatusUnauthorized)
    }

    if err := userCtx.RefreshClaims.DecodeTokenIntoClaims(userCtx.RefreshToken, middleware.Refresh); err != nil {
      slog.Error("decoding refresh token to build an access token", "error", err)
      return ctx.NoContent(http.StatusInternalServerError)
    }

    accessClaims := middleware.JWTClaims{FirstName: userCtx.RefreshClaims.FirstName}
    accessToken, err := accessClaims.NewToken(middleware.Access)
    if err != nil {
      slog.Error("building a new access token from refresh token", "error", err)
      return ctx.NoContent(http.StatusInternalServerError)
    }

    refreshToken, err := accessClaims.NewToken(middleware.Refresh)
    if err != nil {
      slog.Error("rebuilding a refresh token", "error", err)
      return ctx.NoContent(http.StatusInternalServerError)
    }

    userCtx.AccessToken = accessToken
    userCtx.RefreshToken = refreshToken

    if err := h.store.Auth.UpdateRefreshToken(ctx, refreshToken, refreshCookie.Value); err != nil {
      slog.Error("updating the new refresh token", "error", err)
      return ctx.NoContent(http.StatusInternalServerError)
    }

    if err := userCtx.AccessClaims.DecodeTokenIntoClaims(accessToken, middleware.Access); err != nil {
      slog.Error("decoding newly built access token", "error", err)
      return ctx.NoContent(http.StatusInternalServerError)
    }

    accessCookie := userCtx.AccessClaims.BuildCookie(middleware.Access, accessToken)
    refreshCookie := userCtx.RefreshClaims.BuildCookie(middleware.Refresh, refreshToken)
    ctx.SetCookie(accessCookie)
    ctx.SetCookie(refreshCookie)

    slog.Info("generated a new access token", "token", accessToken)
    return nil
  }

  userCtx.AccessToken = accessCookie.Value
  userCtx.RefreshToken = refreshCookie.Value

  if userCtx.AccessClaims.FirstName == "" {
    if err := userCtx.AccessClaims.DecodeTokenIntoClaims(userCtx.AccessToken, middleware.Access); err != nil {
      slog.Error("decoding access token from request", "error", err)
      return ctx.NoContent(http.StatusInternalServerError)
    }

    if err := userCtx.AccessClaims.DecodeTokenIntoClaims(userCtx.RefreshToken, middleware.Refresh); err != nil {
      slog.Error("decoding refresh token from request", "error", err)
      return ctx.NoContent(http.StatusInternalServerError)
    }
  }

  ctx.Set("user", userCtx.AccessClaims)
  slog.Info("set user into context", "user", userCtx.AccessClaims.FirstName)
  return next(ctx)
}
