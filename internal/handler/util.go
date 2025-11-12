package handler

import (
  "fmt"
  "time"

  "github.com/golang-jwt/jwt/v5"
  "github.com/labstack/echo/v4"
)

type CustomClaims struct {
  FirstName string `json:"firstName"`
}

func (c *CustomClaims) GenerateToken() (string, error) {
  now := time.Now()
  exp := now.Add(time.Hour * 24)

  token := jwt.New(jwt.SigningMethodHS256)
  claims := token.Claims.(jwt.MapClaims)

  claims["firstName"] = c.FirstName
  claims["iss"] = "railway-playground"
  claims["iat"] = jwt.NewNumericDate(now)
  claims["exp"] = jwt.NewNumericDate(exp)

  tokenString, err := token.SignedString([]byte("SECRET"))
  if err != nil {
    return "", fmt.Errorf("[SIGNING] %w", err)
  }

  return tokenString, nil
}

func DecodeToken(ctx echo.Context) (*CustomClaims, error) {
  token, exists := ctx.Get("user").(*jwt.Token)
  if !exists {
    return nil, fmt.Errorf("[GET] missing token")
  }

  claims, exists := token.Claims.(jwt.MapClaims)
  if !exists {
    return nil, fmt.Errorf("[CONVERSION] invalid token")
  }

  customClaims := CustomClaims{FirstName: claims["firstName"].(string)}
  return &customClaims, nil
}
