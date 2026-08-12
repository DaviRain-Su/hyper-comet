import ProofForgeV2

namespace Proofship

open ProofForgeV2.Language

-- ProofShip · second vertical — time-locked payout (time-lock-payout).
-- Owner configures unlock height + amount; beneficiary claims once after unlock.
-- Same construct budget as rwa-share-registry (Principal / UInt64 / assert / emit).
-- EVM deploy file: NO invariant/proof declarations.
program TimeLockPayout where
  state owner : Principal
  state beneficiary : Principal
  state unlockHeight : UInt64
  state amount : UInt64
  state claimed : UInt64

  event Claimed(amount : UInt64)

  error TooEarly()
  error AlreadyClaimed()
  error NotBeneficiary()

  init(unlock : UInt64, amt : UInt64) do
    owner := context.caller
    beneficiary := context.caller
    unlockHeight := unlock
    amount := amt
    claimed := 0

  -- Owner-only: assign who may claim after unlock (before first claim).
  entry setBeneficiary(who : Principal) : UInt64 do
    assert context.caller == owner
    assert claimed == 0
    beneficiary := who
    return 1

  -- Beneficiary-only: claim once when block height has reached unlockHeight.
  entry claim() : UInt64 do
    assert context.caller == beneficiary
    assert claimed == 0
    assert unlockHeight <= context.blockHeight
    claimed := amount
    emit Claimed(amount)
    return claimed

  view getAmount() : UInt64 do
    return amount

  view getUnlockHeight() : UInt64 do
    return unlockHeight

  view getClaimed() : UInt64 do
    return claimed

  view isUnlocked() : Bool do
    return unlockHeight <= context.blockHeight

end Proofship
