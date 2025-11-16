package store

import (
  "github.com/jackc/pgx/v5/pgxpool"
  "github.com/labstack/echo/v4"
)

type AuthStore struct {
  Pool *pgxpool.Pool
}

func (s *AuthStore) RefreshTokenExists(ctx echo.Context, token string) (exists bool) {
  statement := `
  SELECT EXISTS (
    SELECT 1 FROM users
    WHERE refresh_token = $1
    AND refresh_token IS NOT NULL
    LIMIT 1
  )
  `

  err := s.Pool.QueryRow(ctx.Request().Context(), statement, token).Scan(&exists)
  if err != nil {
    return false
  }

  return
}

func (s *AuthStore) UpdateRefreshToken(ctx echo.Context, newToken, oldToken string) error {
  statement := `
  UPDATE users
  SET refresh_token = $1
  WHERE refresh_token = $2
  `

  _, err := s.Pool.Exec(ctx.Request().Context(), statement, newToken, oldToken)
  if err != nil {
    return err
  }

  return nil
}

func (s *AuthStore) SetRefreshToken(ctx echo.Context, newToken, name string) error {
  statement := `
  UPDATE users
  SET refresh_token = $1
  WHERE first_name = $2
  `

  _, err := s.Pool.Exec(ctx.Request().Context(), statement, newToken, name)
  if err != nil {
    return err
  }

  return nil
}
