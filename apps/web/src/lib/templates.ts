export type ContractTemplate = {
  id: string;
  module: string;
  title: string;
  titleZh: string;
  blurb: string;
  blurbZh: string;
  prompt: string;
  promptZh: string;
  source: string;
};

export const STATE_CELL_SOURCE = `import ProofForgeV2

namespace Proofship

open ProofForgeV2.Language

program StateCell where
  state count : UInt64

  event Incremented(delta : UInt64)

  init(start : UInt64) do
    count := start

  entry increment(delta : UInt64) : UInt64 do
    count := count + delta
    emit Incremented(delta)
    return count

  view getCount() : UInt64 do
    return count

end Proofship
`;

export const RWA_SOURCE = `import ProofForgeV2

namespace Proofship

open ProofForgeV2.Language

program RwaShareRegistry where
  state owner : Principal
  state totalSupply : UInt64
  state issued : UInt64
  state balance : Map Principal UInt64
  state allowlist : Map Principal UInt64
  state maxPerTx : UInt64
  state windowCap : UInt64
  state windowStart : UInt64
  state windowSpent : UInt64

  event Issued(amount : UInt64)
  event Transferred(amount : UInt64)

  error NotAllowed()
  error InsufficientBalance()

  init(supply : UInt64, perTx : UInt64, window : UInt64) do
    owner := context.caller
    totalSupply := supply
    issued := 0
    balance := Map.empty()
    allowlist := Map.empty()
    maxPerTx := perTx
    windowCap := window
    windowStart := context.blockHeight
    windowSpent := 0

  entry setAllow(who : Principal, ok : UInt64) : UInt64 do
    assert context.caller == owner
    assert ok <= 1
    allowlist[who] := ok
    return ok

  entry issue(to : Principal, amount : UInt64) : UInt64 do
    assert context.caller == owner
    assert issued + amount <= totalSupply
    match balance[to] with
    | Option.some(v) => do
      balance[to] := v + amount
      issued := issued + amount
      emit Issued(amount)
      return issued
    | _ => do
      balance[to] := amount
      issued := issued + amount
      emit Issued(amount)
      return issued

  entry transfer(to : Principal, amount : UInt64) : UInt64 do
    assert amount <= maxPerTx
    match allowlist[to] with
    | Option.some(flag) => do
      assert flag == 1
      if windowStart + 1000 <= context.blockHeight then
        windowStart := context.blockHeight
        windowSpent := 0
      assert windowSpent + amount <= windowCap
      match balance[context.caller] with
      | Option.some(bal) => do
        assert amount <= bal
        match balance[to] with
        | Option.some(tb) => do
          balance[context.caller] := bal - amount
          balance[to] := tb + amount
          windowSpent := windowSpent + amount
          emit Transferred(amount)
          return windowSpent
        | _ => do
          balance[context.caller] := bal - amount
          balance[to] := amount
          windowSpent := windowSpent + amount
          emit Transferred(amount)
          return windowSpent
      | _ => do
        revert InsufficientBalance()
    | _ => do
      revert NotAllowed()

  view balanceOf(who : Principal) : UInt64 do
    match balance[who] with
    | Option.some(v) => do
      return v
    | _ => do
      return 0

  view isAllowed(who : Principal) : Bool do
    match allowlist[who] with
    | Option.some(flag) => do
      return flag == 1
    | _ => do
      return false

  view issuedTotal() : UInt64 do
    return issued

  view policy() : UInt64 do
    return maxPerTx

end Proofship
`;

export const TIMELOCK_SOURCE = `import ProofForgeV2

namespace Proofship

open ProofForgeV2.Language

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

  entry setBeneficiary(who : Principal) : UInt64 do
    assert context.caller == owner
    assert claimed == 0
    beneficiary := who
    return 1

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
`;

export const TEMPLATES: ContractTemplate[] = [
  {
    id: "rwa-share-registry",
    module: "RwaShareRegistry",
    title: "RWA share registry",
    titleZh: "RWA 份额登记",
    blurb: "Allowlist + per-tx cap + rolling window. The competition demo case.",
    blurbZh: "白名单 + 单笔上限 + 滚动窗口。竞赛演示用例。",
    prompt:
      "Build an on-chain RWA share registry. Owner sets total supply, max per transaction, and a rolling window cap. Owner can allowlist principals and issue shares. Transfers must be to an allowlisted address, under the per-tx cap, and within the window cap which resets every 1000 blocks.",
    promptZh:
      "做一个链上 RWA 份额登记：owner 设定总供给、单笔上限和滚动窗口上限；可写入白名单并发行份额；转账必须给白名单地址、不超过单笔上限，窗口每 1000 个区块重置。",
    source: RWA_SOURCE,
  },
  {
    id: "time-lock-payout",
    module: "TimeLockPayout",
    title: "Time-lock payout",
    titleZh: "时间锁支付",
    blurb: "Owner sets unlock height and amount. Beneficiary claims once after unlock.",
    blurbZh: "Owner 设定解锁高度与金额，受益人到期后一次性领取。",
    prompt:
      "Create a time-locked payout. Owner configures unlock block height and amount, can assign a beneficiary before the first claim. Only the beneficiary may claim, once, after the unlock height.",
    promptZh:
      "做一个时间锁支付：owner 配置解锁区块高度和金额，首次领取前可指定受益人；只有受益人可在解锁后领取一次。",
    source: TIMELOCK_SOURCE,
  },
  {
    id: "state-cell",
    module: "StateCell",
    title: "State cell",
    titleZh: "状态单元",
    blurb: "Minimal counter. Good first walk through the gate.",
    blurbZh: "最小计数器。走通门禁的第一步。",
    prompt:
      "Write a minimal StateCell program with a UInt64 count, an increment entry that emits Incremented(delta), and a getCount view.",
    promptZh:
      "写一个最小 StateCell：UInt64 计数、increment 入口发出 Incremented(delta)、以及 getCount 视图。",
    source: STATE_CELL_SOURCE,
  },
];

export function templateById(id: string) {
  return TEMPLATES.find((t) => t.id === id);
}
