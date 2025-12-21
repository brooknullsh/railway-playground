package store

import (
  "context"
  "errors"
  "log/slog"
  "net/http"
  "os"

  "github.com/gofiber/fiber/v3"
  "github.com/jackc/pgx/v5/pgxpool"
)

type Store struct {
  pool *pgxpool.Pool
}

func (this *Store) Close() {
  this.pool.Close()
}

func (this *Store) MutInit() error {
  conn := os.Getenv("DATABASE_URL")
  if conn == "" {
    return errors.New("unset $DATABASE_URL variable")
  }

  config, err := pgxpool.ParseConfig(conn)
  if err != nil {
    return err
  }

  config.MaxConns = 20
  config.MinConns = 5

  ctx := context.Background()
  pool, err := pgxpool.NewWithConfig(ctx, config)
  if err != nil {
    return err
  }

  if err := pool.Ping(ctx); err != nil {
    return err
  }

  this.pool = pool
  return nil
}

func (this *Store) SetRefreshToken(ctx fiber.Ctx, token string, id int) int {
  stmt := `
  UPDATE users
  SET refresh_token = $1
  WHERE id = $2
  `

  tag, err := this.pool.Exec(ctx, stmt, token, id)
  if err != nil {
    slog.Error("setting refresh token", "error", err)
    return http.StatusInternalServerError
  }

  if tag.RowsAffected() <= 0 {
    return http.StatusNotFound
  }

  return http.StatusOK
}
