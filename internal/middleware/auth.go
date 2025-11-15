package middleware

import (
  "fmt"
  "log/slog"
  "net/http"
  "os"
  "time"

  "github.com/brooknullsh/railway-playground/internal/store"
  "github.com/golang-jwt/jwt/v5"
  "github.com/labstack/echo/v4"
)

var AccessName = "access_token"
var RefreshName = "refresh_token"
var JWTIssuer = "railway_playground"
var AccessLifespan = time.Second * 15
var SigningMethod = jwt.SigningMethodHS256
var RefreshLifespan = (time.Hour * 24) * 30

type CustomClaims struct {
  FirstName string `json:"firstName"`
  jwt.RegisteredClaims
}

func (c *CustomClaims) GenerateJWT(lifespan time.Duration) (string, error) {
  now := time.Now()
  expiry := now.Add(lifespan)

  c.RegisteredClaims.Issuer = JWTIssuer
  c.RegisteredClaims.IssuedAt = jwt.NewNumericDate(now)
  c.RegisteredClaims.ExpiresAt = jwt.NewNumericDate(expiry)

  token := jwt.NewWithClaims(SigningMethod, c)
  accessSecret, refreshSecret := secretKeyAsBytes()

  if lifespan == AccessLifespan {
    return token.SignedString(accessSecret)
  } else {
    return token.SignedString(refreshSecret)
  }
}

func (c *CustomClaims) BuildCookie(token string, name string) *http.Cookie {
  return &http.Cookie{
    Path:     "/",
    HttpOnly: true,
    Secure:   true,
    Value:    token,
    Name:     name,
    Expires:  c.RegisteredClaims.ExpiresAt.Time,
  }
}

type UserAuthContext struct {
  AccessToken   string
  RefreshToken  string
  AccessClaims  *CustomClaims
  RefreshClaims *CustomClaims
}

func (c *UserAuthContext) SaveJWTsFromCookies(store *store.Store, ctx echo.Context) error {
  refresh, err := ctx.Cookie(RefreshName)
  if err != nil {
    return fmt.Errorf("unauthorised request in protected route")
  }

  c.RefreshToken = refresh.Value
  access, err := ctx.Cookie(AccessName)
  if err != nil {
    if exists := store.Auth.RefreshTokenExists(ctx, c.RefreshToken); !exists {
      return fmt.Errorf("invalid refresh token with no access token")
    }

    if err := decodeTokenToClaims(c.RefreshToken, "", c.RefreshClaims); err != nil {
      return fmt.Errorf("decoding refresh token to build an access token: %w", err)
    }

    // Using the claims within the refresh token, create both access and refresh
    // JWTs. NOTE: Mutates the tokens within the user context.
    newAccessClaims := CustomClaims{FirstName: c.RefreshClaims.FirstName}
    newAccessTokenString, err := newAccessClaims.GenerateJWT(AccessLifespan)
    if err != nil {
      return fmt.Errorf("building a new access token: %w", err)
    }

    newRefreshTokenString, err := newAccessClaims.GenerateJWT(RefreshLifespan)
    if err != nil {
      return fmt.Errorf("building a refresh token from new access claims: %w", err)
    }

    c.AccessToken = newAccessTokenString
    c.RefreshToken = newRefreshTokenString

    if err := store.Auth.UpdateRefreshToken(ctx, c.RefreshToken, refresh.Value); err != nil {
      return fmt.Errorf("updating refresh token after newly generated access token: %w", err)
    }

    if err := decodeTokenToClaims(c.AccessToken, "", c.AccessClaims); err != nil {
      return fmt.Errorf("decoding newly generated access token: %w", err)
    }

    ctx.SetCookie(c.AccessClaims.BuildCookie(c.AccessToken, AccessName))
    ctx.SetCookie(c.RefreshClaims.BuildCookie(c.RefreshToken, RefreshName))

    slog.Info("generated a new access token", "user", c.AccessClaims.FirstName)
    return nil
  }

  // We can safely save the JWT strings to the user context.
  // 1. No refresh token returns early as the request is unauthenticated
  // 2. No access token with a refresh token means it expired, so using the
  //    refresh token, we have created another one.
  c.AccessToken = access.Value
  c.RefreshToken = refresh.Value

  // If both cookies were in the request, we have yet to decode them/one into
  // their respective claims. NOTE: The jwt.RegisteredClaims struct within each
  // user context claims is not comparable.
  if c.AccessClaims.FirstName == "" {
    if err := decodeTokenToClaims(c.AccessToken, "", c.AccessClaims); err != nil {
      return fmt.Errorf("decoding access token from request: %w", err)
    }
  } else if c.RefreshClaims.FirstName == "" {
    if err := decodeTokenToClaims(c.RefreshToken, "", c.RefreshClaims); err != nil {
      return fmt.Errorf("decoding refresh token from request: %w", err)
    }
  }

  slog.Info("set user into authentication context", "user", c.AccessClaims.FirstName)
  return nil
}

func decodeTokenToClaims(tokenString, tokenKind string, claims *CustomClaims) error {
  claimsValidatior := func(token *jwt.Token) (any, error) {
    if token.Method.Alg() != SigningMethod.Alg() {
      return nil, fmt.Errorf("invalid signing method: %s", token.Header["alg"])
    }

    accessSecret, refreshSecret := secretKeyAsBytes()
    if tokenKind == "access" {
      return accessSecret, nil
    }

    return refreshSecret, nil
  }

  if _, err := jwt.ParseWithClaims(tokenString, claims, claimsValidatior); err != nil {
    return fmt.Errorf("%w", err)
  }

  return nil
}

func secretKeyAsBytes() ([]byte, []byte) {
  var accessSecret, refreshSecret string

  if accessSecret = os.Getenv("JWT_ACCESS_SECRET"); accessSecret == "" {
    slog.Error("JWT_ACCESS_SECRET is unset or empty")
    os.Exit(1)
  }

  if refreshSecret = os.Getenv("JWT_REFRESH_SECRET"); refreshSecret == "" {
    slog.Error("JWT_REFRESH_SECRET is unset or empty")
    os.Exit(1)
  }

  return []byte(accessSecret), []byte(refreshSecret)
}
