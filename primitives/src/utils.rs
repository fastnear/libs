use near_primitives::types::AccountId;
use near_primitives::views::StateChangeValueView;

pub fn state_change_account_id(state_change_value: &StateChangeValueView) -> &AccountId {
    match &state_change_value {
        StateChangeValueView::AccountUpdate { account_id, .. }
        | StateChangeValueView::AccountDeletion { account_id }
        | StateChangeValueView::AccessKeyUpdate { account_id, .. }
        | StateChangeValueView::AccessKeyDeletion { account_id, .. }
        // | StateChangeValueView::GasKeyUpdate { account_id, .. }
        | StateChangeValueView::GasKeyNonceUpdate { account_id, .. }
        // | StateChangeValueView::GasKeyDeletion { account_id, .. }
        | StateChangeValueView::DataUpdate { account_id, .. }
        | StateChangeValueView::DataDeletion { account_id, .. }
        | StateChangeValueView::ContractCodeUpdate { account_id, .. }
        | StateChangeValueView::ContractCodeDeletion { account_id } => account_id,
    }
}
