# Documents

The document header has separate Share, Copy Share Link, and Side Panel buttons.
They are borderless with a soft rounded background on hover. Share opens the
sharing dialog; copying a link is a separate action.

Verify **Public → View** links in a fresh signed-out browser context. The
`/app/drive/md/<id>` route (including folder/tab document links and legacy
`/app/md/<id>` redirects) must show the document without private Drive lists or
folder navigation, on desktop and touch devices. The login banner may remain;
it must not replace the public document. Signed-out readers see **Sign in to view
comments**, not a comment-fetch error or composer. While identity is unresolved,
the discussion shows neither comments nor a sign-in prompt; throttle the preview
auth response to check this on desktop and touch devices. Account-only lists, contacts,
notifications, activity, and gateway/storage WebSockets must stay idle until
sign-in; public document metadata, properties, and sync still load normally.
Check that signing in restores account services and signing out stops them.
Private files still prompt for sign-in. Signed-out Drive list URLs (My Files,
Recent, Shared with me, or folders without a document target) show **Sign in to
view your files** with Login and Sign Up actions, never an indefinite loader.

Opening a private file link while signed in shows **You don't have access to
this file**, the signed-in email, and **Request access**. View is selected by
default; choose View, Comment, or Edit where supported, submit, then verify
**Request sent** and **Cancel request**.
On touch devices, document links open the full-page document route; the same
request controls appear there, including at phone widths.
The document stays unavailable until its owner approves. The open denied view
reloads when it receives the grant update. Signed-out visitors are prompted to
sign in first; ordinary load failures retain **Try again**.
An already-open document reloads after a permission upgrade on desktop and touch
devices. Removing its last grant closes the editor and returns to Request access;
a failed background refresh must not restore the revoked document.

In **People with access**, a non-owner can use **Request access** beneath the
owner to request a supported upgrade (**Comment** or **Edit**). PDFs offer
Comment; view-only email sharing has no upgrade. Requests can be sent or changed
once per minute; cancellation stays available immediately. Approval allows a
further upgrade immediately. After a denial, the requester must wait 24 hours
to request again.
This workflow also applies to the existing chat, folder, project, database, call,
and agent-session Share menus, using the access levels that each supports.

Reviewers see a pending count on **Share**. Entering an item with pending requests
opens Share once for that visit; requests arriving during the visit update the
badge. On mobile, review requests in the **People** tab. **Permission requests**
lets a reviewer choose an access level and **Grant access**, or **Deny**. Team
owners/admins and the human responsible for a bot-owned item review on its behalf.
Only the first review of a request succeeds. Grants notify the requester; denials
do not. Approved users appear in People with access, with controls to change or
remove their direct grant. Opening a permission-request notification navigates
to its item and opens Share.

Both **Edit with AI** and selected-text AI edits open the shared usage-limit dialog
when the backend refuses a request with a usage-limit code. Free users can choose
**View plans**; paid users can choose **Open usage settings** to add credits. The
failed edit leaves the document unchanged and clears its running state. Other edit
failures still show **AI edit failed**. The dialog also opens for usage-limit failures
from live document-editing tools in chat.

Snippet owners manage team sharing under **Share → Team access**. The details
panel has no separate Sharing section. Choose **Edit** to grant the access the
old snippet toggle provided, or **None** to remove team access.

Hover or focus a document title in the header to see its owner, created time,
and last-updated time. This details card is shared
by documents, tasks, snippets, and file blocks; unavailable metadata is
labeled rather than inferred. File breadcrumb names follow live renames,
including the tab title and title menu.

Editable documents show matching **Add tags** and **Add property** pills below
the title. Once tags are applied, **Add tags** becomes the existing tag name or
count pill; clicking it reopens the picker. **Add property** opens the shared
property selector for that document.
Adding or reselecting a property from the title row pins it there. Adding from
the side panel leaves it unpinned. The side panel shows all assigned properties,
including unpinned ones.
Hover or focus an inline property pill to find **Unpin** (keeps its saved
value) and **Delete from item** (removes the document's assignment, without
deleting the shared property definition). Pins are saved with the document.

Markdown code blocks have a **Copy Code** button in both editable and read-only
views. Successful copies briefly animate the icon to a solid green check-circle;
they do not show a success toast.

Embedded code files and PDFs in Markdown documents show their content
below the card header, including when the document opens in a drive detail view.
Project cards remain header-only. To verify, reopen a document with existing
embeds and check that their preview bodies load; also check that a code
mention offers **Convert to Embed** in its preview menu.
Drag an embed's bottom-right resize grip, then release and select the card. Its
height should stay at the dragged size and persist after reopening the document.

## Create a document from an editor

Type `/document` in a Markdown slash menu and choose **Document**. The popover
has a **New document** title, a Markdown body, and a **Tags** pill. It starts
without task property pills. Tags can also be applied with `#` in the title or
body. Tags sit on the left of the bottom row beside **Create Document**.
Enter in the title moves into the body. Opening mentions keeps the composer at
the same height.

Choose **Create Document** or press Cmd/Ctrl+Enter. The composer closes and the
source editor shows a creation placeholder at the insertion point, then replaces
it with the new document mention. If creation fails, the placeholder is removed
and the composer reopens with the title, body, and tags for retry.
**Continue editing in split** creates the document with the current draft and
tags, opens it in an adjacent split, and inserts its mention in the source editor.
An empty title uses **New Note** when expanding.
**Create → Document** continues to open a document directly.

## Markdown outline

On desktop, Markdown documents with at least three headings show a tick rail in
the left margin. The rail appears only where the text column's left margin is
wide enough to hold it clear of the text, so it never overlaps the checkbox
markers of a checklist; a document whose margin comments have pushed the text
column against the container padding shows no rail at any window width. Every
section whose content overlaps the editor viewport is
highlighted, including a section whose heading has already scrolled above it.
Hover a tick (or Tab to its button) to expand it and nearby ticks and show a
rounded preview with the section heading and up to three lines of body text.
While a preview is open, only its tick is emphasized; visible-section highlights
return when the preview closes.
The preview overlays the text column, but it does not take the pointer: a click
that lands on it reaches the document underneath, so checkboxes it covers still
toggle, and moving off the rail closes it even if the pointer crosses the card.
Click a tick or press Enter to jump immediately to its heading without closing its preview
or collapsing the expanded ticks. Scrolling, resizing, and
editor updates refresh the visible-section highlights.

On a local HTTPS stack, document and image downloads use `/local-storage/`
on the app's HTTPS origin. A request to HTTP localhost indicates a stale
storage URL or stack configuration; hard-refresh after updating the stack.
Markdown also needs a successful `/sync/document/.../connect` WebSocket upgrade.
A 403 there indicates the sync origin check, which the local proxy handles for
HTTPS machine hostnames; verify the proxy configuration before retrying.

## Databases in documents

Opening the slash menu with `/` lists Normal Text, headings, and the other
Markdown options before Query and Database. Typing `/database` selects
**Database**: it creates an Untitled database with its first table and Name
column, then inserts its live editor at the command's position. Rename the
database in the embedded header. A failed create clears its pending placeholder;
removing the placeholder while creation runs does not insert elsewhere.

Selecting a database from `@` inserts a compact mention. Use its **Expand database**
icon to open the inline editor. Databases created with `/database` start expanded.
Edit cells, add rows or columns, select a table with the **Database table** picker,
choose a saved view with **Database view** when the table has views, filter, and
sort in the document using the same toolbar controls as the full database app.
The **Filter** and **Sort** icons open their panels. The pickers support keyboard
navigation and mark the selected table. Rows added or edited elsewhere, including
by AI, appear through the database's gateway subscription. The editor catches up after
reconnecting or returning to the tab after a long absence.
Edits write to the original database. **Open** opens the full database.
**Collapse** converts the editor to a small mention; **Expand** brings it back.
Removing either reference leaves the database and its rows intact.

Document permission controls changing the reference. Database permission controls
editing its rows, columns and name, independently of document permission. An
expanded editor in a read-only document can still edit a writable database;
its collapse/expand is local to that reader. A database viewer sees read-only
records. Sharing the document does not itself grant database access. Filters,
sort order and widths are local to this visit; the selected table and view are
saved in the Lexical node when the document is editable and sync to other readers.

## Live database answers

With Databases on, type `/query` and choose **Query** to insert a live answer
to a question about a database. Answers run with each reader's database access and
refresh when referenced tables change. See
[Databases](databases.md#ai-questions-and-live-answers) for the question box, source
picker, displays, and editing.

## Desktop file drops

In a rebuilt macOS desktop app, drag a PNG and a PDF from Finder into a
disposable Markdown document, then into channel and AI chat attachment inputs.
Check that each drop shows the upload feedback and adds each file once, with
images rendered inline in Markdown and attachments visible in the composers.
Try multiple files and a second drop after the first completes. Discard unsent
drafts after checking. Also check that the file picker still attaches files.

Use an actual Finder drag for this check: synthetic browser events bypass
Tauri's native interception and cannot verify the desktop window configuration.
The setting and native rebuild requirement are documented in the
[web agent guide](../../apps/web/AGENTS.md#desktop-file-drops).

## Folder uploads

Dropping a folder (or choosing one with the folder picker) zips it in the browser
and imports it as a folder tree: each subfolder becomes a folder, including empty
ones, and each file becomes a document. The folder picker cannot see empty
subfolders, so only dropped folders keep them. Office lock files (`~$…`),
`Thumbs.db`, `desktop.ini`, `__MACOSX` and hidden files or folders are skipped.
Extension case does not matter (`REPORT.DOCX` imports as `.docx`). If some files
fail, the rest of the tree still appears and a toast says some files could not be
uploaded. The unzip runs in the upload extractor Lambda, which the local stack
does not run; verify folder imports against a deployed backend.

## Legacy Office uploads

Uploaded legacy binary Office files are converted to their OpenXML equivalents
on the server: `.doc` → `.docx`, `.ppt` → `.pptx`, `.xls` → `.xlsx`. This also
applies to files inside an uploaded folder. The document keeps its id and name;
its file type changes and the converted file becomes its latest version.
Macro has no Word, PowerPoint, or Excel editor, so converted files open as
downloadable files (Word documents can preview as PDF when DOCX-to-PDF is enabled).

While a legacy file uploaded in the last 10 minutes is converting, its view shows
**Converting … to a PowerPoint presentation…** (or Word document / Excel
workbook) with **Download** still available for the original. When the
conversion lands, the split reopens in the right block (workbooks switch in
place). If conversion fails or takes over 3 minutes, the view falls back to
**No preview available**, and the original stays downloadable. Legacy files
uploaded before conversion existed are not converted.

Conversion runs in the convert service through the upload finalizer Lambda,
neither of which the local stack runs, so verify it against a deployed backend.

## Create and type

Pasting a Macro `/app/agents/<uuid>` session URL into a Markdown editor converts
it to an agent mention, just like the legacy `/app/agent/<uuid>` URL. Link query
parameters are retained, except for referral codes.

Pasting a routed entity link such as `/app/drive/md/<uuid>` in an app editor
creates the same document mention as a legacy link. Routed links retain the
entity identity and compatible block targets (for example `comment_id`), but
not the workspace path or pane-local search state. A copied multi-pane URL
(`/app/.../~/...`) references its rightmost pane. If that pane has no supported
entity, the URL remains an ordinary link. Project task comment targets, Home PRs,
and agent chat links also convert to their respective entity mentions.

Pasting a bare GitHub pull request URL (`github.com/<owner>/<repo>/pull/<n>`,
including subpages such as `/files`) inserts a link that becomes a PR mention
once Macro finds the synced PR, usually within a moment. Hovering the mention
shows its status, diff size, comments, and checks; clicking opens the PR in a
split. A PR mention always stays on one line: a long title truncates with an
ellipsis before the `#N`, so read the full title from the hover card. In a sent
message, a mention an agent wrote by URL alone shows `owner/repo#N` until its
title loads and keeps the same width either way. A PR Macro has not synced
stays an ordinary link, and pasting inside a code block or over a selection
keeps the default behavior.

1. `Create` → `Document D`. The app navigates to `/app/md/<uuid>` with the **title field
   focused**.
2. `type_text` the title, then `submitKey: "Enter"` to drop into the body.
3. Type paragraphs with plain `type_text`; use Enter between paragraphs. Do NOT use `fill` —
   the editor is contenteditable and `fill` does not work on it.
4. The document auto-saves continuously (collaborative CRDT; no save button). The tab title
   and header update to the typed title.

The a11y snapshot exposes the entire body as the contenteditable's `value` and as paragraph
nodes — use the snapshot itself to verify content. For formatting checks, run
`evaluate_script` over `[contenteditable] strong` etc.

Body placeholder advertises: `/` for block commands, `@` to reference files, `;` for snippets.
Markdown auto-format works while typing (`#` heading, `[]` checklist, `>` quote).

Every top-level checklist shows a completed count and progress meter on a thin
line above its items, left-aligned with the checkboxes. Checklists containing
an actual task mention additionally get a filter menu (status, priority,
assignee, due date) that reveals on hover and stays while filters are active;
filters dim unmatched items in place and persist with the document. Plain
checkbox lists have no filter, and removing the last task mention removes it.
On Android, use the software keyboard to check `:` emoji, `/` commands, `;`
snippets, and `#` tags where enabled. Each should open once and filter as you
type. Tapping an emoji or command applies it; a second `#` closes the tags menu
and leaves literal `##` for Markdown headings.

AI text-writing operations require a paragraph/list-item or text-run ID. A
table, row, cell, or list-container ID is rejected with guidance to choose a
content block or use `setCell`. Existing stray inline content directly inside
table cells is preserved in paragraphs when the editor opens the document.

`@` opens the mention menu wherever the caret starts a word, including directly
in front of existing text — the menu opens empty there instead of searching for
the word ahead of the caret. It also opens immediately after a soft line break:
type a word, press Shift+Enter, then type `@` and a name to filter the menu.
Typed inside a word (`he@llo`) it stays literal text.
On Android, verify this with the software keyboard: tap `@`, type a name to
filter, and tap a result to insert a single mention. The menu should remain
visible above the keyboard while typing.

`Ctrl+F` / `Cmd+F` opens the in-document find bar. Matches include paragraph
text and inline mention chips (tasks, docs, channels, skills, …) by the title
shown on the chip.

On touch devices, the text-selection menu (Copy, Cut, Comment, Share, and other
available actions) appears above the floating header, comment input, and bottom
dock. It stays anchored to the selection while the document scrolls.

On a touch device, swipe a list item right to indent one level (Apple Notes
style) or left to outdent. Nested children move with the parent. The first
item can indent too, even in a single-item list. Vertical scrolling and taps
are unchanged.
Items stay still during the swipe and change indentation only when a
successful swipe is released; short or blocked swipes leave them in place.
Swiping requires permission to edit the document; comment-only access does
not allow indentation changes. Losing edit permission during a swipe cancels it.
To verify nesting, give a list item a child and grandchild, then swipe the
parent right and left: all three should shift one level together, preserving
their relative depths and order.

## CRM company mentions

With CRM enabled, type `@` followed by a company name or domain in an editor or
composer. Companies appear in their own mention bucket. With
`ENABLE_GRAPHQL_SOUP` enabled, results include cached companies even if they are
absent from the first 500 companies in the REST Quick Access feed; the REST feed
remains a fallback. Cache search covers synchronized companies, not the entire CRM.

To verify, search for a cached company absent from that REST page, select it, and
check that the inserted company mention points to the correct company. Also check
searching by domain and that an open picker updates when companies finish hydrating.
Discard unsent test drafts rather than sending them.

## Project mentions

With Projects enabled, type `@` followed by a project name in an editor or
composer. Projects (not folders) come from Quick Access, so
they appear alongside documents and tasks in the **Documents, Agents, & Tasks**
section and in entity property pickers that accept projects. The command menu
keeps its own project search. Selecting one inserts a document mention with the
project's icon and current name, like a channel mention. Clicking it or
pressing Enter on it opens the project the way a task mention opens a task:
in Tasks, under **Projects** › the project (on touch devices, as the project
view on its own). A project you cannot read shows **No Access**. Pasting
`/app/initiative/<id>` or a Tasks project link inserts the same mention.

The mention is stored as
`<m-document-mention>{"documentId":"<initiative id>","blockName":"initiative",…}</m-document-mention>`
(`project` is a folder). In a document it is tracked as a reference like other
entity mentions. It is deliberately not a channel-message reference, so
mentioning a project in a channel never shares the project with the channel's
members.

To verify, mention a project in a document and in a channel draft, check the
mention opens the right project, rename the project and reload to see the name
update, and delete the mention. Discard unsent test drafts rather than sending
them.

## CRM pipeline mentions

Pipelines are not in the `@` menu or the command menu. Pasting a pipeline link
into an editor or composer inserts a document mention with a table icon and the
pipeline's current name: the pipeline's copy-link action
(`/app/companies/pipeline/<id>`), an older shared view link
(`/app/companies?crmView=…` whose view is that pipeline) and
`/app/pipeline/<id>` all work. Copied Customers links for other views
(Companies, People, lists) stay plain links. A pipeline you cannot read shows
**No Access**; a trashed one shows **Deleted**.

Clicking the mention, or pressing Enter on it, opens Customers on the pipeline
at `/app/companies/pipeline/<id>` (on touch devices too), in the mention's split
like other entity mentions. A split already showing that pipeline is brought
forward instead.

The mention is stored as
`<m-document-mention>{"documentId":"<pipeline id>","blockName":"pipeline",…}</m-document-mention>`.
Like a project mention, it is never a channel-message reference, so mentioning
a pipeline in a channel does not share it with the channel's members.

To verify, copy a pipeline's link in Customers, paste it into a document,
check the chip's name, click it with Customers closed and again with Customers
open on Companies, and open `/app/pipeline/<id>` directly.

## CRM associations

With CRM enabled, any task, document or call can point at CRM records through the
`Companies` and `Contacts` system properties: side panel `Properties` →
`Add property`. Both pickers list Quick Access records: the team's companies and
its most recently interacted contacts, filtered by name, domain or email. CRM
contacts have their own Quick Access bucket, apart from people, and are not
offered in `@` mentions or the command menu. Values show the
record's name and open the company or contact. An entity can carry the property
without listing it (set at creation or through the API); adding that property
pins the existing value rather than clearing it. Calls are linked automatically
when they end, from their participants and the invitees of the calendar event
carrying the meeting link; verify on a finished call's `Properties`.

## Native offline reopening

On native mobile, previously opened Markdown documents/tasks can reopen after an
app restart using their cached body and last-known permissions. Warm the document
online first, then restart with API traffic blocked: verify the existing body,
make a disposable edit, and restart offline again to check local recovery.
Restoring connectivity must reauthorize synchronization before queued edits reach
the server; verify the server copy, not just the still-cached editor text.
Also reconnect after the initial sync's 10-second timeout: a reconnect snapshot
must release queued edits without requiring the document to reopen. A document
content-readiness timeout is retryable and must not revoke its cached open
context; explicit access denial still does.
The body should not wait for unrelated CRM metadata, references, duplicate-task
suggestions, closed sharing/tag menus, or disabled mention queries. With those
requests pending, the cached editor remains visible; optional information can
appear when its own request finishes.

For a cold deep link or restored split, document loading waits for persisted
user identity before capturing its offline session; a stalled auth request must
not delay an identity already restored from IndexedDB. If no identity is cached,
the normal auth query must succeed first. A previous logout marker is not a
cached identity: after signing in again, it must trigger fresh authentication,
not clear the new login cookie. Verify this restart/deep-link flow with a
disposable account. Logout during the identity wait prevents the old load from
opening under a subsequent login.

Cached open context is scoped to the signed-in user and invalidated at logout;
permission tokens are never persisted. A missing body snapshot still requires an
online open—metadata alone must not produce an editable empty document. This path
does not imply offline coverage for PDFs, attachments, or other binary files.

## Reference hover previews

The `@` menu includes `Recent agent sessions` after Channels and before
Companies. Search by session or persona name within the 500 most recently
updated accessible sessions. Menu rows show the
session title followed by a muted persona name, including `@Cursor` and
`@Macro` for built-in personas. Names from the session API take precedence;
older responses use the shared built-in name resolver or cached custom bots.
Selecting one inserts an
inline reference showing the shared agent icon and an underlined session title.
Chips omit persona avatars and status. Click it (or select the node and press
Enter) to open `/app/agent/<id>`.
It references an existing session; it does not invoke the persona, attach its
transcript to AI context, or grant access. Private/deleted sessions show an
unavailable label. Mounted references refresh every 30 seconds while the tab is
active to update titles and check access. Expanded session references are block-level
Magic Chips with the same filled background as document cards. Click the card body
to select its editor node (border and selection ring); use **Open session** to navigate.
The collapse action returns the card to an inline session mention. Text before and
after an expanded reference remains in separate paragraphs.

Hover a document reference chip to open its preview without navigating. With
the preview open, the compact header shows a tinted icon, title, and author/time
byline. Click the title to open the document; the Reference actions ellipsis menu contains copy
link, split, embed/collapse, AI, and delete actions when applicable. The preview
stays open while this menu is active, and moving over other reference chips must
not open their previews. Click outside or press Escape to dismiss the menu;
other references can then be hovered again. Images use an inset frame and task chips
appear below the header. Long titles wrap in place without a full-name tooltip.

With `ENABLE_GRAPHQL_SOUP` enabled, the popup reuses the reference's live `ItemPreviews`
batch, including task properties and viewer permission, without another fetch.
Explicit refreshes may revalidate that batch, but requests must settle while the
pointer stays over the same reference; cache updates must not cause a continuous
fetch cascade. Adding or changing a direct sibling of a document reference must
leave the reference mounted without flashing or restarting its preview query.

## Embedded document cards

Document cards use a compact icon/title row and an actions menu. Full previews
sit inside an inset surface; the author's display name and update time appear
under the title as a byline. The plain 1rem icon sits in a column to the left
of the title, aligned with its first line. Wrapped title lines, the byline,
and task chips share the title's left edge. Full previews use the card's full
content width with equal left and right insets. Title and byline share a text
stack with a consistent 4px gap and 20px title leading, including when the title wraps.
Titles and bylines use text-sm, differentiated by semibold and regular weight;
smaller details use text-xs.
Item.Icon provides the plain first-line-aligned icon slot. The small ellipsis button
sits at the top right. Full embeds have a 320px minimum
card height and a smaller rounded inset frame.
The document-preview overlay uses the same plain icon, title/byline stack,
small actions button, and task status control; image previews keep equal side insets.
Metadata-only references omit the preview. Tasks replace the type icon with an
icon-only status control; click it to change status when you have edit access.
Priority and assignee chips remain below, without a duplicate status chip.
Status and detail slots share one TaskPropertiesPreviewProvider per card:
GraphQL preview data is reused, and REST fallback property/access queries are
owned once, not separately by each slot. Non-task cards do not load task properties.
Use the title to open the referenced document and the
actions menu for the same applicable actions as a mention: Copy Link, Ask Macro
for supported document types outside chat, Copy Branch Name for tasks, and
Open in New Split when the reference is not already open. Editable cards also
offer Convert to Inline Mention and Delete; these actions update when the editor
becomes editable after loading. Converting back preserves the referenced document
and its location parameters. Collapse/Expand Reference is only
available on mentions; cards (including sent-message cards) never offer collapse.
Title navigation preserves the reference's block parameters, including message,
thread, annotation, and document locations.
Click the card frame to select its editor node; controls and embedded content
handle their own clicks. Full embeds remain vertically resizable and scroll
inside the inset preview. When verifying, check a code embed, a metadata-only
reference, and an editable task, including resize, menu actions, and keyboard
access to the title and property controls.

## AI edit

1. Click `Edit with AI` (button directly under the editor body).
2. A focused prompt box appears (placeholder `Describe the edit…`). Type the instruction,
   press Enter (or click `Send`).
   Selected-text AI edit uses the shared Send button: muted and disabled for an
   empty or whitespace-only prompt, black with a white arrow in light mode once
   an instruction is entered (inverted for dark mode), on desktop and mobile.
3. While running, the button row shows a `Stop` button (a11y text `Stop AI edit`), and the
   live cursor walking the text is labelled with the editor's name: `Macro (AI)` for an
   inline edit, or the name of the agent or persona whose session asked for the edit
   (e.g. `Grunk (AI)`). Edits stream directly into the document — there is no
   accept/reject step. The editor can insert the same `@` mention chips a person can:
   dates/times, people, documents, channels, agent sessions (including the expanded
   Magic Chip card), and the other chip types.
4. Completion signal: the `Stop` button disappears. Poll for that with `evaluate_script`;
   do not rely on `wait_for` text.

## Comments (Discussion)

Below the editor: `Discussion` section with a `Leave a comment...` contenteditable.
Desktop uses the same compact 15px composer as channels and AI chat, with an
`Attach images` paperclip that opens the image picker directly. Wrapping text or
Shift+Enter expands the editor above the controls. Lists, blockquotes, and other
non-paragraph blocks always expand while editing. A single paragraph returns to
the compact layout once it fits on one line.
Touch keeps separate `Attach images` and
`Format` buttons. `Send comment` is disabled until text exists. Click the
composer, `type_text`, then click `Send comment` (Enter also submits). The comment renders
above the composer with author + timestamp. `@`-mentions in comments notify the mentioned
user. Editing a discussion comment keeps the attachment and send controls, with no
trash button. Deleting a comment's first message deletes the whole discussion —
the confirmation reads `Delete comment`, the replies under it go too, and an
anchored comment's highlight clears from the document. Deleting a reply removes
only that reply. On mobile, the new-comment composer is docked above the navigation bar,
replacing New when commenting is available in documents and tasks.
When the comment composer is unavailable, New remains available; these views
have no floating Ask AI input.
Tap `Leave a comment...`
to expand the channel-style input; use Send comment to submit (Enter inserts a
newline on mobile). Submitting clears and unfocuses the mobile input, returning
it to its compact state and dismissing the keyboard. The compact input's paperclip
opens the native photo library in the iOS app, with a file-picker fallback when
unavailable; browsers use the file picker. Cancelling adds no images.
While the main document editor is focused with the virtual keyboard
open, the floating comment input is hidden; dismissing the keyboard or leaving
the document editor restores it with any unsent draft intact. Comments remain in the
Discussion section, and collapsing that section does not hide the docked composer.
On touch devices, the Discussion section is hidden until it contains a comment;
the floating **Leave a comment...** input remains available. If the discussion
becomes empty again, the section disappears. Desktop keeps the empty section
and inline input.

Comments anchored to selected text open in a floating margin card on desktop and
a `Comments` drawer on touch devices. Hovering a desktop card reveals its actions
without changing the card size or header text wrapping. New comments, replies,
and edits use plain inputs on the card or drawer's background. The pinned reply
keeps at least 16px of
bottom clearance above the drawer's curve, including while the keyboard is open,
and accounts for the home-indicator safe area when the keyboard is closed.

An inline-comment notification or `/app/md/<id>?comment_id=<comment-id>` link
should scroll to the anchor and open its thread on the first click, including
when the document has not loaded yet. Verify both comments arriving before the
editor and comments arriving after it. Once loaded, click elsewhere in the
document, then click the same notification again: it should revisit the comment,
while background comment refreshes should leave the user's position alone.

Verify bottom Discussion links from Home too:
`/app/home/md/<id>?s0.drive.commentId=<comment-id>`. Open a long document from
another Home item with its caches cold. The first click should reach the comment
even when its messages arrive before the editor or loading content interrupts
the initial scroll. Repeat the click after scrolling away. Wheel, touch,
keyboard input, or dragging the scrollbar during navigation should take control
immediately; later background refreshes must not pull the reader back. If the
editor never becomes ready, its pending comment jump expires after ten seconds;
subsequent recovery must not revive it without a new navigation.

A document row keeps pointing at its newest comment notification until that
notification is done, so on desktop the row opens the document at that comment.
On touch the thread is a drawer over the whole page rather than a margin card,
so a row tap opens the document itself: no drawer, with the comment's highlight
still in the text. Verify this from Home, Inbox, and Files with a comment
notification that has already been read — the drawer must not reappear on every
open. Tapping the highlighted text still offers `Show comment`, and opening the
notification itself still opens the thread.

When checking desktop margin placement, scroll a long document while an embed
or image above the highlighted text changes height. Scroll anchoring may keep
the text at the same screen position; its comment card should stay aligned
through the resize, without briefly jumping upward or downward. Repeat with
several rapid height changes, including while scrolling has paused.

Also verify anchored comments in Drive's detail pane: open a document with
existing text anchors, then click a numbered comment badge to expand it. The
document should stay visible and the thread should open; loading the document
with its badges still collapsed does not exercise thread rendering. Comment
copy links should retain the document/task route and the selected comment.

### Document discussions

Document comments are messages read and written through
`/dss/messages/document/<id>`, and both comment surfaces reuse the channel
message components, as channels do.

Below the editor, expand `Discussion` to see comments without a text anchor.
Its `Leave a comment...` composer is the channel composer: the `+` (`Add`) menu,
formatting, mentions, and `Send message` (Enter also submits). Confirm
completion by the new message appearing above the composer; a failed send
retains the draft. The timeline initially loads a bounded page with up to three
preview replies per thread. Expand a thread to load its replies;
`Load earlier comments` pages backward. Live updates preserve unsent replies
and edits while updating the surrounding thread.

Discussion and comment headers include the date for older messages (for example,
`Yesterday at 4:37 PM` or `09/24/26 at 4:37 PM`). Regular channel timelines retain
their date dividers and time-only message headers.

When verifying `@` mentions, compare the same person query in the document body
and the Discussion composer: shared contacts use the same recent-interaction
ranking. The desktop menu keeps up to three People results visible while other
result categories load; use **View all** for the remaining matches. Check that a
person stays clickable after document and email results arrive. Type and
backspace through a query that keeps the same matches: existing rows should
stay mounted and the menu should not collapse while cached results refresh.
Check document titles and their order as well as People: type and backspace
between a name's prefixes and verify the top document does not disappear and
return while the result count briefly drops.
Changing the total number of matches should not change a category's preview
slots when it still has enough rows to fill them; use **View all** for the full list.
Clear the unsent draft after testing.

Select text and choose the comment action to create an anchored comment. These
threads appear beside their text in the margin (or in the active thread drawer
on phones) and never in the bottom Discussion, including after live updates or
reloads. Links to these threads open the margin without changing the bottom
Discussion's timeline or expanding it. Existing highlights locate threads by
their stable mark IDs. Replies, attachments, reactions, and editing use the same
message controls as channels. In every floating comment card (the Markdown,
PDF margins and expanded minimized threads), a long comment folds to
a faded preview with **Show more**; **Show less** folds it again. Short
comments, the phone drawer, the bottom Discussion, and channels always show the
full text. Markdown margin cards stack without overlapping: a tall or expanded card
pushes the cards below it down (past the end of a short document if needed),
and no card starts above the top of the document. Check this with a long
thread active near the bottom of a short document that has another thread
above and below it.
Removing the last marked text moves its retained conversation to Discussion,
where it remains after reload. Removing only part of a marked range keeps the
conversation anchored to the remaining text. On phones, the active Markdown
thread opens in a drawer with a pinned reply composer; long-press any message
for edit, delete, copy-link, and reaction actions.

A document thread carries no thread-level controls above it. Deleting the root
message deletes the whole discussion, replies included, and answers with the
root's tombstone. `Copy link` targets the
specific comment with `comment_id=<message id>`. Previously copied numeric links
still resolve under current document permissions. Deleting an anchored Markdown
discussion removes its mark while preserving the document text and any
overlapping comments. If deletion happens while the document is closed, its next
editable view removes the retained mark when the document loads. Read-only
viewers see plain text without a dead comment highlight; the stored document
and overlapping live comments stay intact.

PDF comment threads in the right margin use the channel composer (`Leave a comment...`, Enter sends) and the message
thread controls. Highlight comments come from selecting text and choosing the
comment button in the selection menu; placeable comments come from the toolbar
`Comment` tool and a click on the page. Discussions read and post through
`/dss/messages/document/<id>`; anchor geometry still loads from
`/dss/annotations/anchors/document/<id>`. Deleting a highlight's discussion keeps the highlight as a plain
highlight; deleting a placeable's discussion removes the placeable. An anchor bound only
to a legacy annotation thread that was never imported stays hidden rather than
shown as a bare highlight.

## PDF signatures

Choose **Signature** in the PDF toolbar, then click the page where it should go.
The **Draw signature** dialog opens over the active PDF. Drag across the canvas
with a mouse, pen, or finger; **Use signature** becomes available after drawing.
Confirm to place the signature, then drag it to adjust its position. **Cancel
signature**, Escape, or clicking outside discards the unfinished placement.
Signatures are saved into that PDF through its normal save flow; reusable saved
signatures are not available yet.

When verifying this flow, draw near the right and bottom edges as well as the
center. Check at desktop and phone widths and at 2x pixel density; strokes should
stay under the pointer, survive resizing, and appear on the PDF after confirming.
Reopen the dialog and verify it stays centered during its opening animation and
starts with an empty canvas. Cancelling must leave no empty signature box.

## Document history

On desktop, open the title's file menu (**…**) and choose **History**. This
opens an overlay filling the current document block, with a read-only version
preview on the left and a timeline graph plus sessions on the right. History is
no longer a side-panel section and its file-menu item is hidden on mobile.
Scrub the graph to preview a point in time, or select a session to see its changes.
**Current version** returns the preview to the live version; **Fork** copies the
selected version into a separate document. **Close history** or Escape returns
to the mounted editor without losing its scroll position. The two columns scroll
independently, and other app splits remain available.

## Side panel

Right side of a doc (toggle with `Hide/Show Side Panel`):

- `Actions` → `Ask Macro` (opens a doc-scoped AI chat, see ai-chat.md), and on
  desktop a `Copy as prompt` pill for documents and tasks. Its primary button
  runs the last-used agent action; the `Agent options` caret lists `Copy as
  prompt`, `MCP setup instructions`, and an `Open in` group (Claude Code Web,
  Codex Desktop, Cursor, Zed). Tasks add `Copy branch name`. A document prompt
  wraps the title and markdown in `<document>` / `<document-content>` tags with
  no branch instructions; the toast reads `Prompt copied to clipboard`. The
  Files-view title menu (three dots beside the breadcrumb) also lists `Copy as
  prompt` directly above `Download` for plain documents.
- `Details` → Owner, Created, Last updated.
- `Tags` → `Add tags` (dialog). Click a tag's label to toggle and save; Shift-click
  keeps the picker open for multiple selections. Reopen it to remove a tag.
  With GraphQL Soup enabled, applying the first tag refetches only that entity's
  properties; later edits update the existing assignment optimistically. Verify
  both the side panel and list-row chip, then reload to confirm persistence.
  This must also work after background backfills populate more than 128 cached
  Soup variants—tag saves must not scan all cached pages.
  `Properties` → `Add property`.
- `Activity` is collapsible; document statistics and ownership timestamps appear in the footer.
- `Activity` lists the same glyph-rail lines as `/app/component/activity` (plain glyphs on a
  thin connector, one line each with long names truncated, compact `17h` / `8d` / `1mo`
  times; consecutive edits fold into one `made 3 edits` line). Past four entries it shows the
  three newest, a `View all activities` toggle row (dotted connector, caret glyph), and the
  oldest fetched entry (usually `created this`) pinned last; the toggle flips to `Show less`
  once expanded.
  Human content edits appear after the next sync flush. Continued editing creates
  one Activity event until that editor has been inactive for five minutes; their
  next edit then starts another event. Opening a document alone creates no edit.
- Header: `Share`, `Copy Share Link`, overflow menu — use `Share` to inspect or change the
  doc's visibility/permissions. Documents, AI chats, and folders have a `Team access`
  control (None / View / Comment / Edit) for sharing directly with the owner's team.
  That is independent of the team-scoped link control. Folders hide link sharing, so
  Team access is its own card on desktop and a Team tab on mobile, not nested in the
  Link card.

## Known failure: "expected instance of LoroDoc"

Opening any doc can crash with a full-screen dialog `expected instance of LoroDoc` (console:
`[observability] expected instance of LoroDoc`). Seen after the Vite dev server reconnects
(HMR leaves two copies of the loro wasm module alive). `Try Again` and a normal reload do NOT
fix it; a **hard reload ignoring cache** (`navigate_page` with `ignoreCache: true`) does.

## HTML preview tabs

HTML documents show `Render` and `Code` inset tabs in the top bar in both
standalone desktop blocks and Drive detail views. Switch to Code to inspect or
edit the source, then back to Render to see the current preview. Other code file
types do not show these tabs.

## Large-document undo checks

In a disposable Markdown document, change several paragraphs in one edit, then
undo and redo. Verify the text, paragraph count, and a second peer agree after
each operation; wait for saving to finish and reload to check persistence.
