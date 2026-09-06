# blocksign-mcp

A [Model Context Protocol](https://modelcontextprotocol.io) (MCP) server for
[**blocksign.red**](https://blocksign.red) — blockchain-anchored e-signatures.

Every signature blocksign produces is anchored to the **XRP Ledger** and
**Bitcoin**, so agreements are *cryptographically provable* — not just asserted
by a database. This MCP server lets AI agents create, send, negotiate, and
verify legally binding agreements.

## Endpoint

- **URL:** `https://mcp.blocksign.red/mcp`
- **Transport:** Streamable HTTP (MCP `2025-06-18`, negotiates down to `2024-11-05`)
- **Auth (account mode):** `Authorization: Bearer bsk_agent_*` — issue an
  agent-scoped key at <https://blocksign.red/settings/api-keys>

## No API key? Guest mode

Agents can drive multi-party signing with **no account and no key** — payment is
the identity. Call `create_agreement` with no `Authorization` header and pass:

- `payment` — a Stripe shared payment token (`spt_*`)
- `sender` — `{ "name": "...", "email": "..." }`
- `document` + 2+ `signers`

blocksign charges the token, emails each signer a click-to-sign link, and
returns the signing URLs + a check-back `status_url` + the public
`verification_url`.

## Tools

| Tool | Description |
|---|---|
| `create_agreement` | One call: template + variables, Markdown, or base64 PDF → send to signers (individuals *or* companies) |
| `get_agreement_status` | Unified status: envelope state, per-signer status, verification URL |
| `list_agreements` | Recent agreements for the calling org |
| `list_templates` | Contract templates (org-owned + platform public) |
| `verify_agreement` | Public, no-auth cryptographic verification of the audit chain + anchors |
| `void_agreement` | Void an envelope before all parties sign |
| `preview_template` | Render a template with variables (validate inputs before sending) |
| `upload_document` | Upload text/Markdown → PDF, optional AI field detection |
| `detect_fields` | AI-detected signature/initials/date field positions |
| `check_billing_status` | Plan, credit balance, usage, per-document pricing |
| `purchase_credits` | Buy a credit pack (10/50/200) |

Agents can also pay autonomously — blocksign supports Stripe shared payment
tokens (`spt_*`) and x402, so a one-call `create_agreement` can create, pay,
and send with no human in the loop.

## Discovery

- MCP metadata: `https://api.blocksign.red/.well-known/mcp.json`
- MCP server card (SEP-1649): `https://api.blocksign.red/.well-known/mcp/server-card.json`
- A2A AgentCard: `https://a2a.blocksign.red/.well-known/agent-card.json`
- `llms.txt`: `https://blocksign.red/llms.txt`

## Quick start

```bash
# Point any MCP client at the hosted endpoint
npx @modelcontextprotocol/inspector https://mcp.blocksign.red/mcp
```

### Add to your MCP client

```json
{
  "mcpServers": {
    "blocksign": {
      "type": "http",
      "url": "https://mcp.blocksign.red/mcp",
      "headers": { "Authorization": "Bearer bsk_agent_YOUR_KEY" }
    }
  }
}
```

(For guest mode, omit the `headers` block and pass `payment` + `sender` in the
`create_agreement` call.)

### Run it yourself

```bash
export BLOCKSIGN_API_URL=http://localhost:8085   # a running blocksign API
export MCP_PUBLIC_URL=http://localhost:8200/mcp
cargo run
```

Then `POST /mcp` with JSON-RPC, sending your `bsk_agent_*` key as
`Authorization: Bearer <key>` (or using guest mode with `payment`).

## About this repo

This is the MCP server component of blocksign.red, published for agent
discoverability. The core platform (REST API, anchoring, worker services)
remains closed-source.
