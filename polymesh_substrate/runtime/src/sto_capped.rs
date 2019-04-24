use crate::utils;
use crate::asset;
use crate::asset::AssetTrait;
use crate::identity;
use crate::identity::IdentityTrait;
use support::traits::Currency;

use rstd::prelude::*;
use runtime_primitives::traits::{As, CheckedAdd, CheckedSub, CheckedMul, CheckedDiv};
use support::{dispatch::Result, StorageMap, StorageValue, decl_storage, decl_module, decl_event, ensure};
use system::{self, ensure_signed};

/// The module's configuration trait.
pub trait Trait: timestamp::Trait + system::Trait + utils::Trait + balances::Trait{
	// TODO: Add other types and constants required configure this module.

	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type Asset: asset::AssetTrait<Self::AccountId,Self::TokenBalance>;
    type Identity: identity::IdentityTrait<Self::AccountId>;

}

#[derive(parity_codec::Encode, parity_codec::Decode, Default, Clone, PartialEq, Debug)]
pub struct STO<U,V,W> {
    beneficiary: U,
    cap: V,
    sold: V,
    rate: u64,
    start_date: W,
    end_date: W,
    active: bool
}

#[derive(parity_codec::Encode, parity_codec::Decode, Default, Clone, PartialEq, Debug)]
pub struct Investment<U,V,W> {
    investor: U,
    amount_payed: V,
    tokens_purchased: V,
    purchase_date: W
}

decl_storage! {
	trait Store for Module<T: Trait> as STOCapped {

        // Tokens can have multiple whitelists that (for now) check entries individually within each other
        StosByToken get(stos_by_token): map (Vec<u8>, u32) => STO<T::AccountId,T::TokenBalance,T::Moment>;

        StoCount get(sto_count): map (Vec<u8>) => u32;
	}
}

decl_module! {
	/// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing events
		// this is needed only if you are using events in your module
		fn deposit_event<T>() = default;

        pub fn launch_sto(origin, _ticker: Vec<u8>, beneficiary: T::AccountId, cap: T::TokenBalance, rate: u64, start_date: T::Moment, end_date: T::Moment) -> Result {
            let sender = ensure_signed(origin)?;
			let ticker = Self::_toUpper(_ticker);
            ensure!(Self::is_owner(ticker.clone(),sender.clone()),"Sender must be the token owner");

            let sto = STO {
                beneficiary,
                cap,
                sold:<T::TokenBalance as As<u64>>::sa(0),
                rate,
                start_date,
                end_date,
                active: true
            };

            let sto_count = Self::sto_count(ticker.clone());
            let new_sto_count = sto_count.checked_add(1).ok_or("overflow in calculating next sto count")?;

            <StosByToken<T>>::insert((ticker.clone(),sto_count), sto);
            <StoCount<T>>::insert(ticker.clone(),new_sto_count);

            runtime_io::print("Capped STOlaunched!!!");

            Ok(())
        }

        pub fn buy_tokens(origin, _ticker: Vec<u8>, sto_id: u32, value: u64 ) -> Result {
            let sender = ensure_signed(origin)?;
			let ticker = Self::_toUpper(_ticker);
            
            //PABLO: TODO: Validate that buyer is whitelisted for primary issuance.
            //PABLO: TODO: Validate we are within the STO start time and end time

            let mut selected_sto = Self::stos_by_token((ticker.clone(),sto_id));

            // Make sure sender has enough balance
            let sender_balance = <balances::Module<T> as Currency<_>>::free_balance(&sender);
            ensure!(sender_balance >= <T::Balance as As<u64>>::sa(value),"Insufficient funds");
            
            //  Calculate tokens to min
            let token_conversion = <T::TokenBalance as As<u64>>::sa(value).checked_mul(&<T::TokenBalance as As<u64>>::sa(selected_sto.rate)).ok_or("overflow in calculating tokens")?;
            selected_sto.sold = selected_sto.sold.checked_add(&token_conversion).ok_or("overflow while calculating tokens sold")?;

            // Make sure there's still an allocation
            ensure!(selected_sto.sold <= selected_sto.cap, "There's not enough tokens");

            // Transfer poly to token owner
            <balances::Module<T> as Currency<_>>::transfer(
                &sender, 
                &selected_sto.beneficiary,
                <T::Balance as As<u64>>::sa(value)
                )?;

            // Mint tokens and update STO
            T::Asset::_mint_from_sto(ticker.clone(), sender, token_conversion);
            <StosByToken<T>>::insert((ticker.clone(),sto_id), selected_sto);
            // PABLO: TODO: Store Investment DATA

            // PABLO: TODO: Emit event

            runtime_io::print("Invested in STO");

            Ok(())
        }

	}
}

decl_event!(
	pub enum Event<T> where AccountId = <T as system::Trait>::AccountId {
        Example(u32, AccountId, AccountId),
	}
);

impl<T: Trait> Module<T> {

    pub fn is_owner(_ticker: Vec<u8>, sender: T::AccountId) -> bool {
			let ticker = Self::_toUpper(_ticker);
			T::Asset::is_owner(ticker.clone(), sender)
		}

    fn _toUpper(_hexArray: Vec<u8>) -> Vec<u8> {
        let mut hexArray = _hexArray.clone();
        for i in &mut hexArray {
                if *i >= 97 && *i <= 122 {
                        *i -= 32;
                }
        }
        return hexArray;
    }

}

/// tests for this module
#[cfg(test)]
mod tests {
	use super::*;

	use runtime_io::with_externalities;
	use primitives::{H256, Blake2Hasher};
	use support::{impl_outer_origin, assert_ok};
	use runtime_primitives::{
		BuildStorage,
		traits::{BlakeTwo256, IdentityLookup},
		testing::{Digest, DigestItem, Header}
	};

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	// For testing the module, we construct most of a mock runtime. This means
	// first constructing a configuration type (`Test`) which `impl`s each of the
	// configuration traits of modules we want to use.
	#[derive(Clone, Eq, PartialEq)]
	pub struct Test;
	impl system::Trait for Test {
		type Origin = Origin;
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type Digest = Digest;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = ();
		type Log = DigestItem;
	}
	impl Trait for Test {
		type Event = ();
	}
	type TransferValidationModule = Module<Test>;

	// This function basically just builds a genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
		system::GenesisConfig::<Test>::default().build_storage().unwrap().0.into()
	}
}
