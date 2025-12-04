package store

import (
  "context"
  "errors"
  "fmt"
  "os"

  "github.com/jackc/pgx/v5/pgxpool"
)

type Store struct {
  pool *pgxpool.Pool
}

func New() (Store, error) {
  var storeZero Store

  var conn string
  if conn = os.Getenv("DATABASE_URL"); conn == "" {
    return storeZero, errors.New("missing DATABASE_URL environment variable")
  }

  config, err := pgxpool.ParseConfig(conn)
  if err != nil {
    return storeZero, fmt.Errorf("creating database configuration: %v", err)
  }

  config.MaxConns = 20
  config.MinConns = 5
  ctx := context.Background()

  pool, err := pgxpool.NewWithConfig(ctx, config)
  if err != nil {
    return storeZero, fmt.Errorf("creating database pool: %v", err)
  }

  if err := pool.Ping(ctx); err != nil {
    return storeZero, fmt.Errorf("pinging database: %v", err)
  }

  return Store{pool}, nil
}
