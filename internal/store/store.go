package store

import (
  "context"
  "fmt"

  "github.com/jackc/pgx/v5/pgxpool"
)

type Store struct {
  Database *pgxpool.Pool
}

func New(ctx context.Context, url string) (*Store, error) {
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
