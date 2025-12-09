package handler

import (
  "fmt"
  "log/slog"
  "net/http"
  "time"

  "github.com/gofiber/fiber/v3"
  "github.com/golang-jwt/jwt/v5"
)

type LoginBody struct {
  Id int `json:"id" validate:"required"`
}

func (this *Handler) Login(ctx fiber.Ctx) error {
  var body LoginBody
  if err := ctx.Bind().Body(&body); err != nil {
    slog.Error("invalid request body", "error", err)
    return ctx.SendStatus(http.StatusBadRequest)
  }

  slog.Info("login request", "id", body.Id)
  if !this.store.RefreshTokenByIdExists(ctx, body.Id) {
    return ctx.SendStatus(http.StatusUnauthorized)
  }

  var userState UserState
  userState.Id = body.Id

  accSecret := fiber.MustGetState[string](ctx.App().State(), accTokenKey)
  refSecret := fiber.MustGetState[string](ctx.App().State(), refTokenKey)

  accClaims, refClaims := userState.NewClaimsPair()
  accToken, refToken := accClaims.IntoToken(accSecret), refClaims.IntoToken(refSecret)

  accClaims.IntoCookie(ctx, accTokenKey, accToken)
  refClaims.IntoCookie(ctx, refTokenKey, refToken)
  return ctx.SendStatus(http.StatusOK)
}

func (this *Handler) AuthMiddleware(ctx fiber.Ctx) error {
  userToken := ctx.Cookies(refTokenKey)
  if userToken == "" {
    return ctx.SendStatus(http.StatusUnauthorized)
  }

  refSecret := fiber.MustGetState[string](ctx.App().State(), refTokenKey)
  claims, okay := decodeToken(userToken, refSecret)
  if !okay {
    return ctx.SendStatus(http.StatusUnauthorized)
  }

  var userState UserState
  userState.Id = claims.Id

  if ctx.Cookies(accTokenKey) == "" {
    accSecret := fiber.MustGetState[string](ctx.App().State(), accTokenKey)
    accClaims, refClaims := userState.NewClaimsPair()
    accToken, refToken := accClaims.IntoToken(accSecret), refClaims.IntoToken(refSecret)

    if code := this.store.SetRefreshTokenById(ctx, refToken, claims.Id); code != http.StatusOK {
      return ctx.SendStatus(code)
    }

    accClaims.IntoCookie(ctx, accTokenKey, accToken)
    refClaims.IntoCookie(ctx, refTokenKey, refToken)
  }

  ctx.App().State().Set("user", userState)
  return ctx.Next()
}

type TokenKind int

const (
  Access TokenKind = iota
  Refresh
)

type UserState struct {
  Id int `json:"id"`
}

func (this *UserState) NewClaimsPair() (Claims, Claims) {
  now := time.Now()
  accExpiry := now.Add(time.Second * 15)
  refExpiry := now.Add((time.Hour * 24) * 30)

  builder := func(exp time.Time) Claims {
    var claims Claims
    claims.UserState = *this
    claims.RegisteredClaims.IssuedAt = jwt.NewNumericDate(now)
    claims.RegisteredClaims.ExpiresAt = jwt.NewNumericDate(exp)

    return claims
  }

  return builder(accExpiry), builder(refExpiry)
}

type Claims struct {
  UserState
  jwt.RegisteredClaims
}

func (this *Claims) IntoToken(secret string) string {
  token, err := jwt.NewWithClaims(jwt.SigningMethodHS256, this).SignedString(secret)
  if err != nil {
    slog.Error("encoding token from claims", "error", err)
    return ""
  }

  return token
}

func (this *Claims) IntoCookie(ctx fiber.Ctx, name, value string) {
  var cookie fiber.Cookie
  cookie.Path = "/"
  cookie.Name = name
  cookie.Value = value
  cookie.Secure = true
  cookie.HTTPOnly = true
  cookie.Expires = this.RegisteredClaims.ExpiresAt.Time

  ctx.Cookie(&cookie)
}

func decodeToken(token string, secret string) (Claims, bool) {
  var claims Claims

  validator := func(token *jwt.Token) (any, error) {
    if token.Method.Alg() != jwt.SigningMethodHS256.Alg() {
      return nil, fmt.Errorf("invalid signing method %s", token.Header["alg"])
    }

    return []byte(secret), nil
  }

  leeway := jwt.WithLeeway(60)
  _, err := jwt.ParseWithClaims(token, &claims, validator, leeway)
  if err != nil {
    slog.Error("decoding token", "error", err)
    return claims, false
  }

  return claims, true
}
