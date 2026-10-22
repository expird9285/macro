# OpenAI ChatGPT Apps Directory Submission

## Submission Overview

This document contains all the information required to submit Macro's MCP server to the OpenAI ChatGPT Apps Directory via the ZIP-based plugin package format.

**Owner**: Eric Hayes (eric@macro.com, GitHub: ehayes2000)
**Submission Date**: October 2026
**Status**: Ready for submission after icon export

## Quick Start

The plugin package is ready at: `services/mcp_service/openai-plugin-package/`

**Build the submission ZIP:**
```bash
cd /workspace/services/mcp_service/openai-plugin-package
./build-package.sh
```

**Upload to OpenAI Portal:**
Upload `services/mcp_service/macro-plugin-v1.0.0.zip` to the [OpenAI Plugin Submission Portal](https://platform.openai.com/plugins)

---

## Plugin Metadata

### Basic Information

- **Plugin Name**: Macro
- **Short Description**: Search, read, and create content across documents, emails, and messages in Macro.
- **Long Description**: Macro is a unified workspace that brings together email, chat, tasks, documents, agents, and CRM in one place. This MCP server provides tools for interacting with your Macro workspace: search across all content, read and create documents, manage channels and messages, access agent sessions, query databases, create calendar events, send emails, and more. Macro enables AI clients to seamlessly work with your company's knowledge base and workflows.
- **Category**: Productivity / Workspace Management
- **Website**: https://macro.com
- **Support URL**: mailto:support@macro.com
- **Privacy Policy URL**: https://macro.com/privacy
- **Terms of Service URL**: https://macro.com/terms

### Logo and Assets

**Logo Files** (need to be prepared in required sizes):
- Source: `/workspace/apps/web/public/macro-favicon.svg` and `/workspace/apps/web/src/components/icon/macro-logo.svg`
- Required sizes for submission:
  - 512x512px PNG (high resolution)
  - 256x256px PNG
  - 128x128px PNG
  - SVG version (any size)

**TODO**: Export logo assets in required sizes and upload to a publicly accessible location or prepare for OpenAI submission portal upload.

---

## MCP Server Details

### Production MCP Server

- **MCP Server URL**: `https://mcp-server.macro.com/mcp`
- **Transport**: Streamable HTTP (SSE)
- **Authentication**: OAuth 2.1 backed by FusionAuth
- **Public Accessibility**: ✅ Yes (deployed to AWS ECS, publicly routable)
- **Domain Verification**: **TODO** - Host token at `https://mcp-server.macro.com/.well-known/openai-apps-challenge`

### Authentication Details

**OAuth 2.1 Configuration**:
- **Authorization Endpoint**: `https://auth.macro.com/oauth2/authorize` (FusionAuth public URL)
- **Token Endpoint**: `https://auth.macro.com/oauth2/token`
- **Client Registration**: Dynamic Client Registration supported via OAuth 2.1
- **Scopes**: Full Macro workspace access (read/write across documents, channels, email, databases)
- **Redirect URI Policy**: Allows `chatgpt.com`, `chat.openai.com` as configured in `DEFAULT_MCP_ALLOWED_REDIRECT_HOSTS`

**Authentication Flow**:
1. MCP client initiates OAuth flow
2. User redirects to browser for Macro login
3. User authorizes the MCP client
4. FusionAuth returns authorization code
5. MCP client exchanges code for access token
6. Subsequent tool calls authenticated via Bearer token

**Credentials Stored**: OAuth credentials encrypted with AES-256 via `McpCredentialsKeySecretName`

### Content Security Policy

**CSP Declaration**: Not currently returning UI components, only tool results. If future versions add UI:
- CSP should be declared via `_meta.ui.csp.frameDomains` in resource metadata
- Currently: No iframe embeds, no UI widgets, tool results only

---

## Tool Inventory

The Macro MCP server exposes a comprehensive set of tools across multiple domains. Each tool has been annotated with the required `readOnlyHint`, `destructiveHint`, and `openWorldHint` boolean values.

### Tool Categories

**Search & Discovery** (Read-Only):
- `ContentSearch` - Search across all content in the workspace
- `NameSearch` - Search entities by name
- `SearchTools` - Discover available tools
- `LoadTools` - Load deferred tool schemas
- `ListEntities` - Browse recent items
- `SearchSkills` - Find skills by name
- `ListSkills` - List available skills
- `SelfKnowledge` - Get information about the AI assistant's capabilities

**Document Operations**:
- `ReadContent` - Read document content (Read-Only)
- `ReadMetadata` - Read document metadata (Read-Only)
- `CreateDocument` - Create new documents (Additive)
- `EditDocument` - Edit existing documents (Destructive)

**Channel & Communication** (Open World where applicable):
- `ReadChannelMessages` - Read messages from channels (Read-Only)
- `ReadChannelThread` - Read threaded conversations (Read-Only)
- `ReadChannelMessageContext` - Get message context (Read-Only)
- `SendChannelMessage` - Send messages to channels (Additive, Open World)
- `ShareWithChannel` - Share content to channels (Additive)
- `CreateChannelMediaUpload` - Upload media to channels (Additive)

**Email** (Open World):
- `ReadEmail` - Read email messages (Read-Only)
- `SendEmail` - Send email messages (Additive, Open World)
- `SearchEmail` - Search email content (Read-Only)

**Calendar & Scheduling**:
- `ListCalendars` - List user's calendars (Read-Only)
- `CreateCalendarEvent` - Create calendar events (Additive)
- `UpdateCalendarEvent` - Modify events (Destructive)
- `DeleteCalendarEvent` - Delete events (Destructive)
- `ListBookingLinks` - List booking links (Read-Only)
- `CreateBookingLink` - Create booking links (Additive)

**Agent Sessions**:
- `ReadAgentSession` - Read agent conversation transcripts (Read-Only)
- `ReadAgentSessionSharing` - Get sharing settings (Read-Only)
- `UpdateAgentSessionSharing` - Modify sharing (Destructive)

**Database & CRM**:
- `DiscoverDatabases` - List available databases (Read-Only)
- `QueryDatabase` - Execute SQL queries (Read-Only for SELECT, Destructive for INSERT/UPDATE/DELETE)
- `ReadCrmContact` - Read CRM contact (Read-Only)
- `CreateCrmContact` - Create contact (Additive)
- `UpdateCrmContact` - Update contact (Destructive)
- `ListCrmPipelines` - List sales pipelines (Read-Only)

**Projects & Organization**:
- `ListProjects` - List projects/folders (Read-Only)
- `CreateProject` - Create new project (Additive)
- `ReadProject` - Read project details (Read-Only)
- `MoveToProject` - Move items (Destructive)
- `MoveToTrash` - Delete items (Destructive)
- `RenameItem` - Rename items (Destructive)

**Skills**:
- `ReadSkill` - Read skill instructions (Read-Only)
- `ListSkills` - List available skills (Read-Only)
- `SearchSkills` - Search for skills (Read-Only)

**Forms**:
- `CreateForm` - Create forms (Additive)
- `SetFormAccess` - Modify form permissions (Destructive)

**Properties & Tags**:
- `GetEntityProperties` - Read properties (Read-Only)
- `SetEntityProperty` - Set properties (Destructive)
- `ListTags` - List tags (Read-Only)
- `CreateTag` - Create tag (Additive)
- `EditTag` - Modify tag (Destructive)
- `DeleteTag` - Delete tag (Destructive)

**Import & Integration**:
- `ImportChannel` - Import Slack/Discord channels (Additive, Open World)
- `DiscoverConnectors` - List available connectors (Read-Only)

**Coding Agents**:
- `ReadCodingAgentSession` - Read coding agent transcripts (Read-Only)
- `ListCodingAgents` - List available coding agents (Read-Only)

### Tool Annotations Summary

All tools in the Macro MCP server implement the required `ToolAnnotated` trait with explicit boolean values:

**readOnlyHint**: Set to `true` for all retrieval, computation, or preview operations (Read*, List*, Search*, Get*, Discover*). Set to `false` for any operation that modifies state (Create*, Edit*, Update*, Delete*, Send*, Set*, Move*).

**destructiveHint**: Set to `false` for read-only tools and additive writes (Create*, Add*). Set to `true` for potentially destructive or irreversible operations (Edit*, Update*, Delete*, Move to trash, Send email/message, Revoke access).

**openWorldHint**: Set to `true` for tools that interact with public internet or send data outside the user's bounded workspace (SendEmail, SendChannelMessage to external channels, Import from external services, web search). Set to `false` for tools confined to the user's private Macro workspace.

**idempotentHint**: Set to `true` for read-only operations. Set appropriately for writes based on whether repeated calls with identical parameters have the same effect.

---

## Test Credentials

**Test Account Requirements**:
- **Demo Account Email**: **TODO** - Create a fully-featured demo account
- **Demo Account Password**: **TODO** - Set a permanent password (no MFA)
- **Account Features**: The demo account should include:
  - Sample documents with varied content
  - Sample email threads
  - Active channels with message history
  - Calendar events
  - CRM contacts and deals
  - Database tables with data
  - Agent session history
  - Booking links
  - Forms

**Access Notes**:
- Account must be accessible without 2FA, SMS codes, or additional verification
- Account should persist indefinitely for OpenAI reviewers
- Account should represent realistic usage with interconnected data

---

## Example Prompts / Starter Prompts

1. "Search my workspace for documents about Q4 planning"
2. "Create a new document called 'Meeting Notes' and add today's standup highlights"
3. "What are the latest messages in the #engineering channel?"
4. "Send an email to the team with this week's progress update"
5. "Show me upcoming calendar events for the next 7 days"
6. "Query the customers database for contacts in California"
7. "Read the most recent agent session about the product launch"
8. "Create a new CRM contact for Acme Corp with contact details"
9. "What skills are available to help with marketing tasks?"
10. "Share the product roadmap document with the #product channel"

---

## Test Cases

### Positive Test Cases (Expected to Succeed)

1. **Search and Read Document**
   - **Prompt**: "Search for documents about 'onboarding' and read the first result"
   - **Expected Behavior**: 
     - `ContentSearch` tool called with query "onboarding"
     - Returns list of matching documents with IDs and snippets
     - `ReadContent` tool called with document ID
     - Returns full markdown content of the document
   - **Expected Output**: Document content displayed with proper formatting

2. **Create and Share Document**
   - **Prompt**: "Create a new document called 'Team Updates October 2026' with the heading 'Weekly Updates', then share it with the #general channel"
   - **Expected Behavior**:
     - `CreateDocument` tool creates document with specified name and content
     - Returns document ID
     - `ShareWithChannel` tool shares document to #general channel
   - **Expected Output**: Confirmation that document was created and shared successfully

3. **Read Channel Messages**
   - **Prompt**: "What are the last 5 messages in the #engineering channel?"
   - **Expected Behavior**:
     - `ReadChannelMessages` tool called with channel identifier and limit=5
     - Returns message history with sender, timestamp, and content
   - **Expected Output**: List of 5 recent messages with metadata

4. **Calendar Event Creation**
   - **Prompt**: "Create a calendar event for 'Sprint Planning' tomorrow at 2pm for 1 hour"
   - **Expected Behavior**:
     - `CreateCalendarEvent` tool called with event details
     - Event created in user's default calendar
   - **Expected Output**: Confirmation with event ID and calendar link

5. **Database Query**
   - **Prompt**: "Query the contacts database for all people with email addresses from @acme.com"
   - **Expected Behavior**:
     - `QueryDatabase` tool called with SQL query filtering by email domain
     - Returns result rows matching the criteria
   - **Expected Output**: Table of contacts with Acme Corp email addresses

### Negative Test Cases (Expected to Fail Gracefully)

1. **Invalid Document ID**
   - **Prompt**: "Read document with ID 00000000-0000-0000-0000-000000000000"
   - **Expected Behavior**:
     - `ReadContent` tool called with invalid/non-existent document ID
     - Tool returns error: "Document not found" or "Access denied"
   - **Expected Output**: Clear error message without exposing internal details

2. **Permission Denied**
   - **Prompt**: "Delete the document 'Company Financials Q4 2026'" (assuming user lacks permission)
   - **Expected Behavior**:
     - Tool attempts to delete document
     - Returns permission error if user lacks delete access
   - **Expected Output**: "You don't have permission to delete this document" or similar

3. **Malformed Query**
   - **Prompt**: "Query the database with SQL: SELECT * FROM nonexistent_table"
   - **Expected Behavior**:
     - `QueryDatabase` tool executes query
     - Database returns error for non-existent table
   - **Expected Output**: SQL error message: "Table does not exist" without crashing

---

## Domain Verification

**TODO - Required Action**: Host the domain verification token at:
```
https://mcp-server.macro.com/.well-known/openai-apps-challenge
```

**Implementation**: Add an endpoint in `services/mcp_service/src/main.rs` or via nginx/CDN that serves the OpenAI-provided challenge token as plain text at the well-known path.

**Verification Steps**:
1. Obtain verification token from OpenAI submission portal
2. Create endpoint or static file at `.well-known/openai-apps-challenge`
3. Verify token is accessible via HTTPS GET request
4. Confirm in submission portal that verification succeeds

---

## Reviewer Notes

### Architecture Overview

Macro's MCP server is a Rust-based service deployed on AWS ECS Fargate, serving tools via the Streamable HTTP (SSE) transport specified by the MCP protocol. Authentication is handled through OAuth 2.1 backed by FusionAuth, with credentials encrypted using AES-256.

The service is built on a hexagonal architecture with clear separation between domain logic, inbound adapters (HTTP/MCP transport), and outbound adapters (database, external services). All tools are implemented in their respective domain crates and assembled into a unified toolset in the `ai_tools` crate.

### Key Technical Details

- **Language**: Rust (stable)
- **Transport**: Streamable HTTP (rmcp library)
- **Authentication**: OAuth 2.1 with FusionAuth
- **Database**: PostgreSQL (MacroDB)
- **Telemetry**: OpenTelemetry with Datadog integration
- **Deployment**: AWS ECS Fargate behind Application Load Balancer
- **Autoscaling**: Configured for 1-10 tasks (dev: 1-3) based on CPU/memory/request count

### Privacy & Security

- OAuth credentials are encrypted at rest using AES-256
- All tool operations are scoped to the authenticated user's permissions
- Implements standard entity access control via `entity_access` crate
- Complies with Macro's privacy policy at https://macro.com/privacy
- ISO 27001 and SOC 2 certified
- Tool responses minimize data collection and return only request-relevant information

### Tool Response Format

All tools return structured JSON responses that include:
- Primary result data
- Optional `_meta` fields for UI rendering hints
- Media attachments (images, videos) with inline data URIs or downloadable URLs
- Markdown formatting for rich text content
- Links to Macro web app items (documents, channels, contacts) using the `APP_BASE_URL` config

### Known Limitations

- No UI widgets currently implemented (tool results only)
- Some legacy tools from chat interface not exposed to MCP
- Subagent tool not available to MCP clients (recursive agent creation restricted)
- DisplayResults tool omitted for MCP (clients handle their own display)

---

## Pre-Submission Checklist

- [ ] **Organization Verification**: Complete identity verification in OpenAI Platform Dashboard
  - Individual verification for eric@macro.com OR
  - Business verification for Macro Inc.
- [ ] **API Permissions**: Ensure `api.apps.write` and `api.apps.read` permissions granted
- [ ] **Domain Verification**: Host challenge token at `/.well-known/openai-apps-challenge`
- [ ] **Logo Assets**: Export and upload logo in required sizes (512px, 256px, 128px, SVG)
- [ ] **Test Account**: Create and configure fully-featured demo account with no MFA
- [ ] **Test Credentials**: Document test account email and password
- [ ] **Privacy Policy**: Verify https://macro.com/privacy is accessible and current
- [ ] **Terms of Service**: Verify https://macro.com/terms is accessible and current
- [ ] **Support Contact**: Confirm support@macro.com is monitored
- [ ] **Tool Scanning**: Run `Scan Tools` in submission portal and verify all annotations
- [ ] **Test Cases**: Validate all 5 positive and 3 negative test cases in production
- [ ] **Example Prompts**: Test all 10 starter prompts for correctness

---

## Submission Instructions

### Step 1: Pre-Submission Preparation

1. Complete organization verification in [OpenAI Platform Dashboard](https://platform.openai.com)
2. Export logo assets in required sizes
3. Create demo account with sample data
4. Implement domain verification endpoint
5. Deploy any required changes to production

### Step 2: Portal Submission

1. Navigate to [Plugin Submission Portal](https://developers.openai.com/plugins)
2. Click "Create Plugin" or "Submit Plugin"
3. Enter MCP Server URL: `https://mcp-server.macro.com/mcp`
4. Configure OAuth 2.1 authentication settings
5. Click "Scan Tools" to import tool metadata
6. Review discovered tools and annotations
7. Fill in all required fields:
   - Plugin name: Macro
   - Short description (see above)
   - Long description (see above)
   - Category: Productivity
   - Logo upload
   - Website, support, privacy, terms URLs
   - Test credentials
   - Example prompts
   - Test cases (5 positive, 3 negative)
   - Localization: English (US)
8. Confirm all checkboxes
9. Submit for review

### Step 3: Post-Submission

1. Monitor email for review updates
2. Respond to any feedback or requests for changes
3. If rejected, review feedback, make changes, and resubmit
4. If approved, click "Publish" to make plugin available in directory
5. Monitor user reports and maintain plugin per OpenAI requirements

---

## Maintenance Plan

### Continuous Review

OpenAI will periodically scan the MCP server and compare tools with published definitions. Changes are automatically reviewed:

- **New tools**: Available after passing automated checks
- **Updated tools**: Previous definition stays live until update passes checks
- **Deleted tools**: Removed immediately upon detection

### Version Updates

For changes requiring new plugin version submission:
- Updated plugin metadata (name, description, logo)
- MCP server origin change (scheme, hostname, or port)
- Major tool contract changes

For changes handled via continuous review:
- Individual tool additions, updates, or removals
- Tool schema changes
- Tool annotation updates
- Server instructions updates
- CSP changes (if UI added in future)

### Monitoring

- Monitor MCP server health via Datadog
- Track tool call success/failure rates
- Monitor user feedback via OpenAI plugin directory
- Respond to reported issues within 48 hours
- Keep demo account active and accessible

---

## Contact Information

**Primary Contact**:
- Name: Eric Hayes
- Email: eric@macro.com
- GitHub: ehayes2000
- Role: Technical Lead

**Company**:
- Name: Macro Inc.
- Website: https://macro.com
- Support: support@macro.com
- Location: San Francisco, CA

**Engineering**:
- MCP Service Source: `/workspace/services/mcp_service/`
- Infrastructure: `/workspace/infra/stacks/mcp-server/`
- Tool Implementations: `/workspace/crates/ai_tools/`

---

## Additional Resources

- [Macro Documentation](https://docs.macro.com)
- [MCP Setup Guide](https://docs.macro.com/AI/mcp/overview)
- [OpenAI Apps SDK Documentation](https://developers.openai.com/apps-sdk)
- [Plugin Guidelines](https://developers.openai.com/plugins/plugin-guidelines)
- [MCP Protocol Specification](https://modelcontextprotocol.io)

---

**Last Updated**: October 21, 2026
**Version**: 1.0
**Status**: Ready for submission after completing TODO items
