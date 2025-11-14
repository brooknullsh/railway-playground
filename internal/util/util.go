package util

import (
  "fmt"
  "log/slog"
  "os"
  "time"

  "github.com/golang-jwt/jwt/v5"
  "github.com/labstack/echo/v4"
)

var JWTLifespan = time.Hour * 24

type CustomClaims struct {
  FirstName string `json:"firstName"`
}

func (c *CustomClaims) GenerateJWT() (string, error) {
  now := time.Now()
  expiry := now.Add(JWTLifespan)

  token := jwt.New(jwt.SigningMethodHS256)
  // Ignoring the error for parsing claims on a fresh token.
  claims := token.Claims.(jwt.MapClaims)

  claims["firstName"] = c.FirstName
  claims["iss"] = "railway-playground"
  claims["iat"] = jwt.NewNumericDate(now)
  claims["exp"] = jwt.NewNumericDate(expiry)

  secret := SecretKeyAsBytes()
  return token.SignedString(secret)
}

func DecodeJWTFromRequest(ctx echo.Context) (*CustomClaims, error) {
  token, exists := ctx.Get("user").(*jwt.Token)
  if !exists {
    return nil, fmt.Errorf("missing token")
  }

  claims, exists := token.Claims.(jwt.MapClaims)
  if !exists {
    return nil, fmt.Errorf("invalid token format")
  }

  return &CustomClaims{FirstName: claims["firstName"].(string)}, nil
}

func SecretKeyAsBytes() []byte {
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
