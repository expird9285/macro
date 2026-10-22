/** Block types accepted by the application. Keep this module free of BlockLoader imports. */
export const BlockRegistry = [
  'call',
  'calendar',
  'chat',
  'database',
  // A questionnaire whose answers are rows of a database table.
  'form',
  'pdf',
  'md',
  'code',
  'image',
  'channel',
  'project',
  'unknown',
  'video',
  'email',
  'contact',
  'company',
  'routine',
  'pr',
  'agent',
  // A task project (`project` is a folder).
  'initiative',
  // A CRM pipeline, shown in the Customers view.
  'pipeline',
] as const;

/** Aliases for block types that share a concrete block implementation. */
export const BlockAliasRegistry = ['csv', 'task', 'snippet', 'skill'] as const;
