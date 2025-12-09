package store

import (
  "log/slog"
  "net/http"

  "github.com/gofiber/fiber/v3"
)

func (this *Store) RefreshTokenByIdExists(ctx fiber.Ctx, id int) (exists bool) {
  stmt := `
  SELECT EXISTS (
    SELECT 1 FROM users
    WHERE id = $1
  )
  `

  err := this.pool.QueryRow(ctx, stmt, id).Scan(&exists)
  if err != nil {
    slog.Error("[STORE] refresh token check", "error", err)
    return false
  }

  return
}

func (this *Store) SetRefreshTokenById(ctx fiber.Ctx, token string, id int) int {
  stmt := `
  UPDATE users
  SET refresh_token = $1
  WHERE id = $2
  `

  _, err := this.pool.Exec(ctx, stmt, token, id)
  if err != nil {
    slog.Error("[STORE] setting refresh token", "error", err)
    return http.StatusInternalServerError
  }

  return http.StatusOK
}
