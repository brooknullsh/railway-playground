package store

import (
  "database/sql"
  "errors"
  "log/slog"
  "net/http"
  "time"

  "github.com/gofiber/fiber/v3"
  "github.com/jackc/pgx/v5"
)

type User struct {
  Id           int            `db:"id"            json:"id"`
  Age          int            `db:"age"           json:"age"`
  IsPro        bool           `db:"is_pro"        json:"isPro"`
  Mobile       string         `db:"mobile"        json:"mobile"`
  LastName     string         `db:"last_name"     json:"lastName"`
  FirstName    string         `db:"first_name"    json:"firstName"`
  CreatedAt    time.Time      `db:"created_at"    json:"createdAt"`
  UpdatedAt    time.Time      `db:"updated_at"    json:"updatedAt"`
  RefreshToken sql.NullString `db:"refresh_token" json:"refreshToken"`
}

func (this *Store) GetUserByName(ctx fiber.Ctx, name string) (*User, int) {
  statement := `
  SELECT * FROM users
  WHERE first_name = $1
  LIMIT 1
  `

  rows, _ := this.pool.Query(ctx, statement, name)
  defer rows.Close()

  user, err := pgx.CollectOneRow(rows, pgx.RowToStructByName[User])
  if err != nil {
    if errors.Is(err, pgx.ErrNoRows) {
      return nil, http.StatusNotFound
    }

    slog.Error("finding user by name", "name", name, "error", err)
    return nil, http.StatusInternalServerError
  }

  return &user, http.StatusOK
}
