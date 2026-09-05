# blocksign-mcp

A [Model Context Protocol](https://modelcontextprotocol.io) (MCP) server for
[**blocksign.red**](https://blocksign.red) — blockchain-anchored e-signatures.

Every signature blocksign produces is anchored to the **XRP Ledger** and
**Bitcoin**, so agreements are *cryptographically provable* — not just asserted
by a database. This MCP server lets AI agents create, send, negotiate, and
verify legally binding agreements in a single call.

## Endpoint

- **URL:** `https://mcp.blocksign.red/mcp`
- **Transport:** Streamable HTTP (MCP `2025-06-18`, negotiates down to `2024-11-05`)
- **Auth:** `Authorization: Bearer bsk_agent_*` — issue an agent-scoped key at
  <https://blocksign.red/settings/api-keys>

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

### Run it yourself

```bash
export BLOCKSIGN_API_URL=http://localhost:8085   # a running blocksign API
export MCP_PUBLIC_URL=http://localhost:8200/mcp
cargo run
```

Then `POST /mcp` with JSON-RPC, sending your `bsk_agent_*` key as
`Authorization: Bearer <key>`.

## About this repo

This is the MCP server component of blocksign.red, published for agent
discoverability. The core platform (REST API, anchoring, worker services)
remains closed-source.
