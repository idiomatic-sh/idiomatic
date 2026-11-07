import { Record } from './types';
interface Local { id: number }
interface Stored<T> {
  record: Record;
  items: Array<Record>;
  extra: T;
  local: Local;
  other: Missing;
  q: ns.Record;
}
