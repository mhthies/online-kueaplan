
ALTER TABLE public.rooms ALTER COLUMN event_id DROP DEFAULT;
ALTER TABLE public.event_passphrases ALTER COLUMN event_id DROP DEFAULT;
ALTER TABLE public.entries ALTER COLUMN event_id DROP DEFAULT;
ALTER TABLE public.categories ALTER COLUMN event_id DROP DEFAULT;
ALTER TABLE public.announcements ALTER COLUMN event_id DROP DEFAULT;

DROP SEQUENCE IF EXISTS public.rooms_event_id_seq;
DROP SEQUENCE IF EXISTS public.event_passphrases_event_id_seq;
DROP SEQUENCE IF EXISTS public.entries_event_id_seq;
DROP SEQUENCE IF EXISTS public.categories_event_id_seq;
DROP SEQUENCE IF EXISTS public.announcements_event_id_seq;
