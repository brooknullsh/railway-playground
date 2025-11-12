package handler

import (
  "fmt"
  "log/slog"
  "os"
  "time"

  "github.com/golang-jwt/jwt/v5"
  "github.com/labstack/echo/v4"
)

var jwtDuration = time.Hour * 24

type CustomClaims struct {
  FirstName string `json:"firstName"`
}

func (c *CustomClaims) GenerateToken() (string, error) {
  now := time.Now()
  exp := now.Add(jwtDuration)

  token := jwt.New(jwt.SigningMethodHS256)
  // Ignoring the error for parsing claims on a fresh token.
  claims := token.Claims.(jwt.MapClaims)

  claims["firstName"] = c.FirstName
  claims["iss"] = "railway-playground"
  claims["iat"] = jwt.NewNumericDate(now)
  claims["exp"] = jwt.NewNumericDate(exp)

  secret := SecretKeyBytes()
  return token.SignedString(secret)
}

func DecodeToken(ctx echo.Context) (*CustomClaims, error) {
  tokenCookie, _ := ctx.Cookie("jwt")
  slog.Info(tokenCookie.Value)

  token, exists := ctx.Get("user").(*jwt.Token)
  if !exists {
    return nil, fmt.Errorf("[DECODE_TOKEN] missing token for < %s >", ctx.Request().URL)
  }

  claims, exists := token.Claims.(jwt.MapClaims)
  if !exists {
    return nil, fmt.Errorf("[DECODE_TOKEN] invalid token format")
  }

  return &CustomClaims{FirstName: claims["firstName"].(string)}, nil
}

func SecretKeyBytes() []byte {
  if envSecret, exists := os.LookupEnv("JWT_SECRET"); exists {
    return []byte(envSecret)
  } else {
    slog.Error("[SECRET] JWT_SECRET is unset")
    os.Exit(1)
  }

  // Unreachable due to the panic call above.
  return nil
}
