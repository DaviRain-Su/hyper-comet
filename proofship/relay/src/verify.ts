import { recoverMessageAddress } from "viem";
import { normalizeAddress } from "./siwe";

export async function recoverSiweSigner(
  message: string,
  signature: string,
): Promise<string | null> {
  if (!signature.startsWith("0x") || signature.length < 132) return null;
  try {
    const recovered = await recoverMessageAddress({
      message,
      signature: signature as `0x${string}`,
    });
    return normalizeAddress(recovered);
  } catch {
    return null;
  }
}
