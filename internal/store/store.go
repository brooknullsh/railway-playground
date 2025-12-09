package store

import (
  "context"
  "errors"
  "os"

  "github.com/jackc/pgx/v5/pgxpool"
)

type Store struct {
  pool *pgxpool.Pool
}

func (this *Store) Close() {
  this.pool.Close()
}

func New() (Store, error) {
  var store Store

  var uri string
  if uri = os.Getenv("DATABASE_URL"); uri == "" {
    return store, errors.New("unset $DATABASE_URL")
  }

  config, err := pgxpool.ParseConfig(uri)
  if err != nil {
    return store, err
  }

  config.MaxConns = 20
  config.MinConns = 5

  ctx := context.Background()
  pool, err := pgxpool.NewWithConfig(ctx, config)
  if err != nil {
    return store, err
  }

  if err := pool.Ping(ctx); err != nil {
    return store, err
  }

  store.pool = pool
  return store, nil
}
