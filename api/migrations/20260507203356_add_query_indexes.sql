CREATE INDEX expenses_group_spent_idx ON expenses (group_id, spent_on DESC, created_at DESC)
    WHERE deleted_at IS NULL;
CREATE INDEX expenses_paid_by_idx ON expenses (paid_by);
CREATE INDEX expense_shares_member_idx ON expense_shares (member_id);
CREATE INDEX settlements_group_idx ON settlements (group_id, settled_on DESC)
    WHERE deleted_at IS NULL;
CREATE INDEX activity_group_created_idx ON activity (group_id, created_at DESC);
