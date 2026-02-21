-- Deleting a group cascades to its members, expenses and settlements, but the
-- references from shares, payers and settlements to members had no ON DELETE
-- action, so deleting any group with expenses failed. Members with history
-- are never deleted on their own (they are marked as left), so cascading is
-- only reached through a group delete.
ALTER TABLE expense_shares
    DROP CONSTRAINT expense_shares_member_id_fkey,
    ADD CONSTRAINT expense_shares_member_id_fkey
        FOREIGN KEY (member_id) REFERENCES members (id) ON DELETE CASCADE;

ALTER TABLE expenses
    DROP CONSTRAINT expenses_paid_by_fkey,
    ADD CONSTRAINT expenses_paid_by_fkey
        FOREIGN KEY (paid_by) REFERENCES members (id) ON DELETE CASCADE;

ALTER TABLE settlements
    DROP CONSTRAINT settlements_from_member_fkey,
    ADD CONSTRAINT settlements_from_member_fkey
        FOREIGN KEY (from_member) REFERENCES members (id) ON DELETE CASCADE,
    DROP CONSTRAINT settlements_to_member_fkey,
    ADD CONSTRAINT settlements_to_member_fkey
        FOREIGN KEY (to_member) REFERENCES members (id) ON DELETE CASCADE;
