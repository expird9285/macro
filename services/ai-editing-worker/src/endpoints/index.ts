import { Hono } from 'hono';
import type { Bindings } from '../env';
import commentMark from './comment-mark';
import edit from './edit';
import forms from './forms';
import traces from './traces';

const endpoints = new Hono<{ Bindings: Bindings }>();

endpoints.route('/edit', edit);
endpoints.route('/forms', forms);
endpoints.route('/comment-mark', commentMark);
endpoints.route('/traces', traces);

export default endpoints;
