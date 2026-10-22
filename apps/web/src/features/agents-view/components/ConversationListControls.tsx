import {
  type ListControlOption,
  ListFilterDropdown,
  type ListFilterGroup,
  ListGroupDropdown,
} from '@app/components/view-shell';
import { cn } from '@ui';
import {
  activeFilterCount,
  type ConversationFilterCategory,
  type ConversationFilters,
  type ConversationGrouping,
  type StatusFilter,
} from '../core/conversation-filters';

type FilterValue = StatusFilter;

const statusDot = (class_: string) => () => (
  <span class={cn('size-1.5 rounded-full', class_)} />
);

const STATUS_OPTIONS: ListControlOption<StatusFilter>[] = [
  { id: 'waiting', label: 'Waiting for you', icon: statusDot('bg-warning') },
  { id: 'working', label: 'Working', icon: statusDot('bg-accent') },
  { id: 'unread', label: 'Unread', icon: statusDot('bg-accent') },
  {
    id: 'idle',
    label: 'Idle',
    icon: () => (
      <span class="size-1.5 rounded-full border border-ink-extra-muted" />
    ),
  },
];

const FILTER_GROUPS: ListFilterGroup<
  ConversationFilterCategory,
  FilterValue
>[] = [
  { id: 'status', label: 'Status', options: STATUS_OPTIONS },
];

const GROUPING_OPTIONS: ListControlOption<ConversationGrouping>[] = [
  { id: 'none', label: 'None' },
  { id: 'status', label: 'Status' },
];

export type ConversationListControlsProps = {
  filters: ConversationFilters;
  grouping: ConversationGrouping;
  onFiltersChange: (filters: ConversationFilters) => void;
  onGroupingChange: (grouping: ConversationGrouping) => void;
};

export function ConversationListControls(props: ConversationListControlsProps) {
  const toggleFilter = (
    category: ConversationFilterCategory,
    value: FilterValue
  ) => {
    const current = props.filters[category];
    const next = current.includes(value)
      ? current.filter((item) => item !== value)
      : [...current, value];
    props.onFiltersChange({ ...props.filters, [category]: next });
  };

  const clearCategory = (category: ConversationFilterCategory) => {
    props.onFiltersChange({ ...props.filters, [category]: [] });
  };

  const clearAll = () => {
    props.onFiltersChange({ status: [] });
  };

  return (
    <div class="flex items-center gap-1">
      <ListFilterDropdown
        groups={FILTER_GROUPS}
        selected={(category) => props.filters[category]}
        activeCount={activeFilterCount(props.filters)}
        onToggle={toggleFilter}
        onClearCategory={clearCategory}
        onClearAll={clearAll}
      />
      <ListGroupDropdown
        options={GROUPING_OPTIONS}
        value={props.grouping}
        onChange={props.onGroupingChange}
      />
    </div>
  );
}
