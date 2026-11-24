import type { ValidateWorkflowResponse } from "@forge/types";

export type ForgeClientOptions = {
  baseUrl: string;
  fetch?: typeof fetch;
};

export class ForgeApiClient {
  private readonly baseUrl: string;
  private readonly fetchImpl: typeof fetch;

  constructor(options: ForgeClientOptions) {
    this.baseUrl = options.baseUrl.replace(/\/$/, "");
    this.fetchImpl = options.fetch ?? fetch;
  }

  async validateWorkflow(yaml: string): Promise<ValidateWorkflowResponse> {
    const response = await this.fetchImpl(
      `${this.baseUrl}/api/v1/workflows/validate`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ yaml }),
      },
    );

    if (!response.ok) {
      throw new Error(`validate workflow failed: ${response.status}`);
    }

    return response.json() as Promise<ValidateWorkflowResponse>;
  }
}