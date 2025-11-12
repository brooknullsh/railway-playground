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
    return nil, fmt.Errorf("[ENV] DATABASE_URL unset")
  }

  config, err := pgxpool.ParseConfig(url)
  if err != nil {
    return nil, fmt.Errorf("[PARSING] %w", err)
  }

  config.MaxConns = 20
  config.MinConns = 5

  database, err := pgxpool.NewWithConfig(ctx, config)
  if err != nil {
    return nil, fmt.Errorf("[CREATING] %w", err)
  }

  if err := database.Ping(ctx); err != nil {
    return nil, fmt.Errorf("[PINGING] %w", err)
  }

  return &Store{database}, nil
}

type User struct {
  Id        int    `db:"id"         json:"id"`
  Age       int    `db:"age"        json:"age"`
  IsPro     bool   `db:"is_pro"     json:"isPro"`
  Mobile    string `db:"mobile"     json:"mobile"`
  LastName  string `db:"last_name"  json:"lastName"`
  FirstName string `db:"first_name" json:"firstName"`
}

func (s *Store) GetUserByName(ctx context.Context, firstName string) (*User, error) {
  row, err := s.Pool.Query(ctx, "SELECT * FROM users WHERE first_name = $1", firstName)
  if err != nil {
    return nil, fmt.Errorf("[QUERYING] %w", err)
  }

  user, err := pgx.CollectOneRow(row, pgx.RowToStructByName[User])
  if err != nil {
    return nil, fmt.Errorf("[MAPPING] %w", err)
  }

  return &user, nil
}
