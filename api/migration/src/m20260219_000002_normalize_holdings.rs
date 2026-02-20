use sea_orm_migration::prelude::*;

/// Data migration: normalize existing raw on-chain token quantities in account holdings.
///
/// Prior to this migration, EVM connector stored raw on-chain integers (e.g., Wei amounts)
/// in the `quantity` field of account holdings JSON. This migration converts those raw values
/// to normalized (human-readable) decimal strings using the `decimals` metadata that is
/// already present in each holding entry.
///
/// # Safety
/// Only holdings that have a `decimals` field set AND whose `quantity` contains no decimal
/// point (i.e., is a raw integer) are touched. Holdings with decimal points (already
/// normalized, e.g., from OKX) or without a `decimals` field are left unchanged.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Normalize raw integer quantities to decimal values for holdings that have
        // a `decimals` field and whose `quantity` is a raw integer (no decimal point).
        db.execute_unprepared(
            r#"
DO $$
DECLARE
    acc RECORD;
    updated_holdings jsonb;
    h jsonb;
    new_h jsonb;
    i int;
BEGIN
    FOR acc IN
        SELECT id, holdings::jsonb AS holdings
        FROM accounts
        WHERE holdings IS NOT NULL
          AND jsonb_array_length(holdings::jsonb) > 0
    LOOP
        updated_holdings := '[]'::jsonb;

        FOR i IN 0 .. jsonb_array_length(acc.holdings) - 1 LOOP
            h := acc.holdings -> i;

            -- Only normalize if decimals is set and quantity looks like a raw integer
            -- (contains no decimal point).
            IF (h -> 'decimals') IS NOT NULL
               AND jsonb_typeof(h -> 'decimals') != 'null'
               AND (h ->> 'quantity') IS NOT NULL
               AND strpos(h ->> 'quantity', '.') = 0
            THEN
                -- Format string supports up to 18 decimal places, which covers all standard
                -- EVM tokens (ETH=18, USDC=6, etc.). Tokens with >18 decimals are non-standard
                -- and not currently supported by the EVM connector.
                new_h := h || jsonb_build_object(
                    'quantity',
                    trim(
                        trailing '0' from
                        trim(
                            trailing '.' from
                            to_char(
                                (h ->> 'quantity')::numeric / power(10, (h ->> 'decimals')::int),
                                'FM999999999999999999999999999990.999999999999999999'
                            )
                        )
                    )
                );

                -- Also normalize 'available' if present and raw integer
                IF (h -> 'available') IS NOT NULL
                   AND jsonb_typeof(h -> 'available') != 'null'
                   AND strpos(h ->> 'available', '.') = 0
                THEN
                    new_h := new_h || jsonb_build_object(
                        'available',
                        trim(
                            trailing '0' from
                            trim(
                                trailing '.' from
                                to_char(
                                    (h ->> 'available')::numeric / power(10, (h ->> 'decimals')::int),
                                    'FM999999999999999999999999999990.999999999999999999'
                                )
                            )
                        )
                    );
                END IF;
            ELSE
                new_h := h;
            END IF;

            updated_holdings := updated_holdings || jsonb_build_array(new_h);
        END LOOP;

        UPDATE accounts SET holdings = updated_holdings WHERE id = acc.id;
    END LOOP;
END $$;
            "#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // The down migration cannot safely reverse this transformation because
        // we don't retain the original raw values. A rollback should be paired
        // with re-syncing accounts via the account sync job to repopulate
        // holdings with fresh (now normalized) data from the connectors.
        Ok(())
    }
}
