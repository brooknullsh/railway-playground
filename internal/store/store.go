package store

import (
  "context"
  "errors"
  "fmt"
  "os"

  "github.com/jackc/pgx/v5/pgxpool"
)

type Store struct {
  User *UserStore
  pool *pgxpool.Pool
}

func NewAndConnect() (*Store, error) {
  var connection string
  if connection = os.Getenv("DATABASE_URL"); connection == "" {
    return nil, errors.New("missing DATABASE_URL environment variable")
  }

  config, err := pgxpool.ParseConfig(connection)
  if err != nil {
    return nil, fmt.Errorf("creating database configuration: %v", err)
  }

  config.MaxConns = 20
  config.MinConns = 5
  ctx := context.Background()

  pool, err := pgxpool.NewWithConfig(ctx, config)
  if err != nil {
    return nil, fmt.Errorf("creating database pool: %v", err)
  }

  if err := pool.Ping(ctx); err != nil {
    return nil, fmt.Errorf("pinging database: %v", err)
  }

  return &Store{&UserStore{pool}, pool}, nil
}
