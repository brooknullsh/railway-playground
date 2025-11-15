package store

import (
  "fmt"
  "log/slog"

  "github.com/jackc/pgx/v5/pgxpool"
  "github.com/labstack/echo/v4"
)

type AuthStore struct {
  Pool *pgxpool.Pool
}

func (s *AuthStore) RefreshTokenExists(ctx echo.Context, refreshToken string) (exists bool) {
  statement := `
  SELECT EXISTS (
    SELECT 1 FROM users
    WHERE refresh_token = $1
    AND refresh_token IS NOT NULL
    LIMIT 1
  )
  `

  err := s.Pool.QueryRow(ctx.Request().Context(), statement, refreshToken).Scan(&exists)
  if err != nil {
    slog.Error("validating refresh token exists", "error", err)
    return false
  }

  return
}

func (s *AuthStore) UpdateRefreshToken(ctx echo.Context, newRefreshToken, oldRefreshToken string) error {
  statement := `
  UPDATE users
  SET refresh_token = $1
  WHERE refresh_token = $2
  `

  _, err := s.Pool.Exec(ctx.Request().Context(), statement, newRefreshToken, oldRefreshToken)
  if err != nil {
    return fmt.Errorf("updating the refresh token: %w", err)
  }

  return nil
}

func (s *AuthStore) SetRefreshToken(ctx echo.Context, newRefreshToken, userFirstName string) error {
  statement := `
  UPDATE users
  SET refresh_token = $1
  WHERE first_name = $2
  `

  _, err := s.Pool.Exec(ctx.Request().Context(), statement, newRefreshToken, userFirstName)
  if err != nil {
    return fmt.Errorf("setting refresh token for [%s]: %w", userFirstName, err)
  }

  return nil
}
