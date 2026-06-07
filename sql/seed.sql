CREATE TABLE IF NOT EXISTS customers (
     id bigserial PRIMARY KEY,
     email text NOT NULL
);

CREATE TABLE IF NOT EXISTS orders (
     id bigserial PRIMARY KEY,
     amount numeric NOT NULL,
     status text DEFAULT 'pending',
     created_at timestamptz NOT NULL DEFAULT now(),
     customer_id bigint REFERENCES customers(id)
);

CREATE INDEX IF NOT EXISTS order_status_idx ON orders (status);