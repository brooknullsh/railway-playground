package store

import (
  "database/sql"
  "time"

  "github.com/jackc/pgx/v5"
  "github.com/jackc/pgx/v5/pgxpool"
  "github.com/labstack/echo/v4"
)

type UserStore struct {
  Pool *pgxpool.Pool
}

type User struct {
  Id           int            `db:"id"         json:"id"`
  Age          int            `db:"age"        json:"age"`
  IsPro        bool           `db:"is_pro"     json:"isPro"`
  Mobile       string         `db:"mobile"     json:"mobile"`
  LastName     string         `db:"last_name"  json:"lastName"`
  FirstName    string         `db:"first_name" json:"firstName"`
  CreatedAt    time.Time      `db:"created_at" json:"createdAt"`
  UpdatedAt    time.Time      `db:"updated_at" json:"updatedAt"`
  RefreshToken sql.NullString `db:"refresh_token" json:"refreshToken"`
}

func (s *UserStore) GetUserByName(ctx echo.Context, name string) (*User, error) {
  statement := `
  SELECT * FROM users
  WHERE first_name = $1
  LIMIT 1
  `
  row, _ := s.Pool.Query(ctx.Request().Context(), statement, name)
  defer row.Close()

  user, err := pgx.CollectOneRow(row, pgx.RowToStructByName[User])
  if err != nil {
    return nil, err
  }

  return &user, nil
}
