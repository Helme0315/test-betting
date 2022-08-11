import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { TestBetting } from "../target/types/test_betting";

describe("test-betting", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.TestBetting as Program<TestBetting>;

  it("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.methods.initialize().rpc();
    console.log("Your transaction signature", tx);
  });
});
