package middleware

import (
  "errors"
  "fmt"
  "log/slog"
  "net/http"
  "os"
  "time"

  "github.com/golang-jwt/jwt/v5"
)

var AccessCookieName = "access_token"
var RefreshCookieName = "refresh_token"

type TokenKind int

const (
  Access TokenKind = iota
  Refresh
)

type JWTClaims struct {
  FirstName string `json:"firstName"`
  jwt.RegisteredClaims
}

func (c *JWTClaims) NewToken(kind TokenKind) (string, error) {
  var lifetime time.Duration
  if kind == Access {
    lifetime = time.Second * 15
  } else {
    lifetime = (time.Hour * 24) * 30
  }

  now := time.Now()
  expiry := now.Add(lifetime)

  c.RegisteredClaims.Issuer = "railway-playground"
  c.RegisteredClaims.IssuedAt = jwt.NewNumericDate(now)
  c.RegisteredClaims.ExpiresAt = jwt.NewNumericDate(expiry)

  token := jwt.NewWithClaims(jwt.SigningMethodHS256, c)
  secret := tokenSecretBytes(kind)

  signedToken, err := token.SignedString(secret)
  if err != nil {
    return "", err
  }

  return signedToken, nil
}

func (c *JWTClaims) BuildCookie(kind TokenKind, token string) *http.Cookie {
  var name string
  if kind == Access {
    name = AccessCookieName
  } else {
    name = RefreshCookieName
  }

  return &http.Cookie{
    Path:     "/",
    HttpOnly: true,
    Secure:   true,
    Value:    token,
    Name:     name,
    Expires:  c.RegisteredClaims.ExpiresAt.Time,
  }
}

func (c *JWTClaims) DecodeTokenIntoClaims(token string, kind TokenKind) error {
  if _, err := jwt.ParseWithClaims(token, c, func(token *jwt.Token) (any, error) {
    if token.Method.Alg() != jwt.SigningMethodHS256.Alg() {
      return nil, fmt.Errorf("invalid signing method %s", token.Header["alg"])
    }

    return tokenSecretBytes(kind), nil
  }); err != nil && !errors.Is(err, jwt.ErrTokenExpired) {
    return err
  }

  return nil
}

func (c *JWTClaims) NeedsRefresh() bool {
  return time.Until(c.RegisteredClaims.ExpiresAt.Time).Minutes() < time.Duration.Minutes(10)
}

type UserContext struct {
  AccessToken   string
  RefreshToken  string
  AccessClaims  *JWTClaims
  RefreshClaims *JWTClaims
}

func tokenSecretBytes(kind TokenKind) []byte {
  var key string
  if kind == Access {
    key = "JWT_ACCESS_SECRET"
  } else {
    key = "JWT_REFRESH_SECRET"
  }

  var secret string
  if secret = os.Getenv(key); secret == "" {
    slog.Error("unset environment variable", "key", key)
    os.Exit(1)
  }

  return []byte(secret)
}
