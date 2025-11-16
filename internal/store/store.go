package store

import (
  "context"
  "errors"
  "os"

  "github.com/jackc/pgx/v5/pgxpool"
)

type Store struct {
  Pool *pgxpool.Pool
  User *UserStore
  Auth *AuthStore
}

func NewAndConnect() (*Store, error) {
  var connString string
  if connString = os.Getenv("DATABASE_URL"); connString == "" {
    return nil, errors.New("unset DATABASE_URL environment variable")
  }

  config, err := pgxpool.ParseConfig(connString)
  if err != nil {
    return nil, err
  }

  config.MaxConns = 20
  config.MinConns = 5

  poolCtx := context.Background()
  pool, err := pgxpool.NewWithConfig(poolCtx, config)
  if err != nil {
    return nil, err
  }

  if err := pool.Ping(poolCtx); err != nil {
    return nil, err
  }

  return &Store{pool, &UserStore{pool}, &AuthStore{pool}}, nil
}
