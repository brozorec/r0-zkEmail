// src/config.ts
export const config = {
  network: "testnet",
  rpcUrl: "https://soroban-testnet.stellar.org",
  networkPassphrase: "Test SDF Network ; September 2015",
  nativeTokenContract: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",

  // Smart Account Kit config
  accountWasmHash: "1fcf7b001c78efeebff016a2a835c2a617b4c1b61dfdb86d5e3bbad31c4c9988",
  webauthnVerifierAddress: "CBEO6Q7UXBIQIHQR42RXETMYDKW7GABRX2O4UVW6O6YQOHROYWZJCOXZ",

  // Contract addresses
  usdcContractAddress: "CBKASTI5ATUW5BJOOCDYINSVOCXXGWAZNXE3MGQCCYZG5MKJOPIR5JVU",
  paymentGatewayAddress: "<CONFIGURE_GATEWAY_CONTRACT>",

  // App config
  checkEmail: "check@mailspay.cc",
  appName: "Email Pay"
} as const;
