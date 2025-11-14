package store

import (
  "context"
  "fmt"
  "os"

  "github.com/jackc/pgx/v5"
  "github.com/jackc/pgx/v5/pgxpool"
)

type Store struct {
  Pool *pgxpool.Pool
}

func NewAndConnect() (*Store, error) {
  ctx := context.Background()

  url, exists := os.LookupEnv("DATABASE_URL")
  if !exists {
    return nil, fmt.Errorf("DATABASE_URL is unset")
  }

  config, err := pgxpool.ParseConfig(url)
  if err != nil {
    return nil, fmt.Errorf("building pool: %w", err)
  }

  config.MaxConns = 20
  config.MinConns = 5

  pool, err := pgxpool.NewWithConfig(ctx, config)
  if err != nil {
    return nil, fmt.Errorf("creating pool: %w", err)
  }

  if err := pool.Ping(ctx); err != nil {
    return nil, fmt.Errorf("pinging database: %w", err)
  }

  return &Store{pool}, nil
}

type User struct {
  Id        int    `db:"id"         json:"id"`
  Age       int    `db:"age"        json:"age"`
  IsPro     bool   `db:"is_pro"     json:"isPro"`
  Mobile    string `db:"mobile"     json:"mobile"`
  LastName  string `db:"last_name"  json:"lastName"`
  FirstName string `db:"first_name" json:"firstName"`
}

func (s *Store) GetUserByName(ctx context.Context, name string) (*User, error) {
  row, _ := s.Pool.Query(ctx, "SELECT * FROM users WHERE first_name = $1", name)

  user, err := pgx.CollectOneRow(row, pgx.RowToStructByName[User])
  if err != nil {
    return nil, fmt.Errorf("collecting single user [%s]: %w", name, err)
  }

  return &user, nil
}
