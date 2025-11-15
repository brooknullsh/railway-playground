package middleware

import (
  "fmt"
  "log/slog"
  "net/http"
  "os"
  "time"

  "github.com/golang-jwt/jwt/v5"
  "github.com/labstack/echo/v4"
)

var JWTName = "jwt"
var JWTLifespan = time.Minute * 5
var JWTIssuer = "railway-playground"

type CustomClaims struct {
  FirstName string `json:"firstName"`
  jwt.RegisteredClaims
}

func (c *CustomClaims) GenerateJWT() (string, error) {
  now := time.Now()
  expiry := now.Add(JWTLifespan)

  c.Issuer = JWTIssuer
  c.IssuedAt = jwt.NewNumericDate(now)
  c.ExpiresAt = jwt.NewNumericDate(expiry)

  token := jwt.NewWithClaims(jwt.SigningMethodHS256, c)
  secret := secretKeyAsBytes()

  return token.SignedString(secret)
}

func (c *CustomClaims) BuildCookie(token string) *http.Cookie {
  return &http.Cookie{
    Path:     "/",
    HttpOnly: true,
    Secure:   true,
    Value:    token,
    Name:     JWTName,
    Expires:  c.ExpiresAt.Time,
  }
}

func (c *CustomClaims) RefreshCookie() (*http.Cookie, int) {
  token, err := c.GenerateJWT()
  if err != nil {
    slog.Error("token refresh", "error", err)
    return nil, http.StatusInternalServerError
  }

  return c.BuildCookie(token), http.StatusOK
}

func DecodeJWTFromMiddleware(ctx echo.Context) (*CustomClaims, error) {
  cookie, err := ctx.Cookie(JWTName)
  if err != nil {
    return nil, fmt.Errorf("missing token in protected route")
  }

  claims := CustomClaims{}
  if _, err := jwt.ParseWithClaims(cookie.Value, &claims, func(_ *jwt.Token) (any, error) {
    return secretKeyAsBytes(), nil
  }); err != nil {
    return nil, fmt.Errorf("decoding token: %w", err)
  }

  return &claims, nil
}

func secretKeyAsBytes() []byte {
  if secret, exists := os.LookupEnv("JWT_SECRET"); exists {
    return []byte(secret)
  } else {
    slog.Error("JWT_SECRET is unset")
    // Exit on startup during handler initialisation.
    os.Exit(1)
  }

  // Unreachable due to the exit call above.
  return nil
}
