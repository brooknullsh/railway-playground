package store

import (
  "context"
  "fmt"

  "github.com/jackc/pgx/v5"
  "github.com/jackc/pgx/v5/pgxpool"
)

type UserStore struct {
  Pool *pgxpool.Pool
}

func (s *UserStore) GetUserByName(ctx context.Context, name string) (*User, error) {
  row, _ := s.Pool.Query(ctx, "SELECT * FROM users WHERE first_name = $1", name)
  defer row.Close()

  user, err := pgx.CollectOneRow(row, pgx.RowToStructByName[User])
  if err != nil {
    return nil, fmt.Errorf("collecting single user [%s]: %w", name, err)
  }

  return &user, nil
}
