CREATE TABLE IF NOT EXISTS webhook_deliveries (
    organization_id TEXT NOT NULL,
    delivery_id TEXT NOT NULL,
    event TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    received_at TEXT NOT NULL,
    PRIMARY KEY (organization_id, delivery_id),
    FOREIGN KEY (organization_id) REFERENCES organizations(id)
);

CREATE INDEX IF NOT EXISTS idx_webhook_org_received
    ON webhook_deliveries (organization_id, received_at);
