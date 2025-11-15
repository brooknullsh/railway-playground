package store

import (
  "context"
  "database/sql"
  "fmt"
  "os"
  "time"

  "github.com/jackc/pgx/v5/pgxpool"
)

type Store struct {
  pool *pgxpool.Pool
  User *UserStore
  Auth *AuthStore
}

func (s *Store) Close() {
  s.pool.Close()
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

  return &Store{pool, &UserStore{pool}, &AuthStore{pool}}, nil
}

type User struct {
  Id        int    `db:"id"         json:"id"`
  Age       int    `db:"age"        json:"age"`
  IsPro     bool   `db:"is_pro"     json:"isPro"`
  Mobile    string `db:"mobile"     json:"mobile"`
  LastName  string `db:"last_name"  json:"lastName"`
  FirstName string `db:"first_name" json:"firstName"`
  // TODO: Omit these?
  CreatedAt    time.Time      `db:"created_at" json:"createdAt"`
  UpdatedAt    time.Time      `db:"updated_at" json:"updatedAt"`
  RefreshToken sql.NullString `db:"refresh_token" json:"refreshToken"`
}
