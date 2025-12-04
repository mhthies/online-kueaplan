--
-- PostgreSQL database dump
--

\restrict 3wy3LqJmoibxMMyHQiicIQF8XxmAUHcRMGhq9kkNfcYfQgAZoEzwMdM4WN8OLU3

-- Dumped from database version 18.1
-- Dumped by pg_dump version 18.1

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET transaction_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

ALTER TABLE ONLY public.rooms DROP CONSTRAINT rooms_event_id_fkey;
ALTER TABLE ONLY public.previous_dates DROP CONSTRAINT previous_dates_entry_id_fkey;
ALTER TABLE ONLY public.previous_date_rooms DROP CONSTRAINT previous_date_rooms_room_id_fkey;
ALTER TABLE ONLY public.previous_date_rooms DROP CONSTRAINT previous_date_rooms_previous_date_id_fkey;
ALTER TABLE ONLY public.events DROP CONSTRAINT events_subsequent_event_id_fkey;
ALTER TABLE ONLY public.events DROP CONSTRAINT events_preceding_event_id_fkey;
ALTER TABLE ONLY public.event_passphrases DROP CONSTRAINT event_passphrases_event_id_fkey;
ALTER TABLE ONLY public.event_passphrases DROP CONSTRAINT event_passphrases_derivable_from_passphrase_fkey;
ALTER TABLE ONLY public.entry_rooms DROP CONSTRAINT entry_rooms_room_id_fkey;
ALTER TABLE ONLY public.entry_rooms DROP CONSTRAINT entry_rooms_entry_id_fkey;
ALTER TABLE ONLY public.entries DROP CONSTRAINT entries_event_id_fkey;
ALTER TABLE ONLY public.entries DROP CONSTRAINT entries_category_fkey;
ALTER TABLE ONLY public.categories DROP CONSTRAINT categories_event_id_fkey;
ALTER TABLE ONLY public.announcements DROP CONSTRAINT announcements_event_id_fkey;
ALTER TABLE ONLY public.announcement_rooms DROP CONSTRAINT announcement_rooms_room_id_fkey;
ALTER TABLE ONLY public.announcement_rooms DROP CONSTRAINT announcement_rooms_announcement_id_fkey;
ALTER TABLE ONLY public.announcement_categories DROP CONSTRAINT announcement_categories_category_id_fkey;
ALTER TABLE ONLY public.announcement_categories DROP CONSTRAINT announcement_categories_announcement_id_fkey;
DROP TRIGGER sync_lastmod ON public.rooms;
DROP TRIGGER sync_lastmod ON public.previous_dates;
DROP TRIGGER sync_lastmod ON public.entries;
DROP TRIGGER sync_lastmod ON public.categories;
DROP TRIGGER sync_lastmod ON public.announcements;
DROP INDEX public.rooms_event_id_title_idx;
DROP INDEX public.previous_dates_entry_id_idx;
DROP INDEX public.event_passphrases_event_id_passphrase_idx;
DROP INDEX public.entries_event_id_begin_idx;
DROP INDEX public.categories_event_id_sort_key_idx;
DROP INDEX public.announcements_event_id_sort_key_idx;
ALTER TABLE ONLY public.rooms DROP CONSTRAINT rooms_pkey;
ALTER TABLE ONLY public.previous_dates DROP CONSTRAINT previous_dates_pkey;
ALTER TABLE ONLY public.previous_date_rooms DROP CONSTRAINT previous_date_rooms_pkey;
ALTER TABLE ONLY public.events DROP CONSTRAINT events_pkey;
ALTER TABLE ONLY public.event_passphrases DROP CONSTRAINT event_passphrases_pkey;
ALTER TABLE ONLY public.entry_rooms DROP CONSTRAINT entry_rooms_pkey;
ALTER TABLE ONLY public.entries DROP CONSTRAINT entries_pkey;
ALTER TABLE ONLY public.categories DROP CONSTRAINT categories_pkey;
ALTER TABLE ONLY public.announcements DROP CONSTRAINT announcements_pkey;
ALTER TABLE ONLY public.announcement_rooms DROP CONSTRAINT announcement_rooms_pkey;
ALTER TABLE ONLY public.announcement_categories DROP CONSTRAINT announcement_categories_pkey;
ALTER TABLE ONLY public.__diesel_schema_migrations DROP CONSTRAINT __diesel_schema_migrations_pkey;
ALTER TABLE public.events ALTER COLUMN id DROP DEFAULT;
ALTER TABLE public.event_passphrases ALTER COLUMN id DROP DEFAULT;
DROP TABLE public.rooms;
DROP TABLE public.previous_dates;
DROP TABLE public.previous_date_rooms;
DROP SEQUENCE public.events_id_seq;
DROP TABLE public.events;
DROP SEQUENCE public.event_passphrases_id_seq;
DROP TABLE public.event_passphrases;
DROP TABLE public.entry_rooms;
DROP TABLE public.entries;
DROP TABLE public.categories;
DROP TABLE public.announcements;
DROP TABLE public.announcement_rooms;
DROP TABLE public.announcement_categories;
DROP TABLE public.__diesel_schema_migrations;
DROP FUNCTION public.sync_lastmod();
--
-- Name: sync_lastmod(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.sync_lastmod() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    NEW.last_updated := NOW();

    RETURN NEW;
END;
$$;


SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: __diesel_schema_migrations; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.__diesel_schema_migrations (
    version character varying(50) NOT NULL,
    run_on timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL
);


--
-- Name: announcement_categories; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.announcement_categories (
    announcement_id uuid NOT NULL,
    category_id uuid NOT NULL
);


--
-- Name: announcement_rooms; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.announcement_rooms (
    announcement_id uuid NOT NULL,
    room_id uuid NOT NULL
);


--
-- Name: announcements; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.announcements (
    id uuid NOT NULL,
    event_id integer NOT NULL,
    announcement_type integer NOT NULL,
    text character varying NOT NULL,
    show_with_days boolean NOT NULL,
    begin_date date,
    end_date date,
    show_with_categories boolean NOT NULL,
    show_with_all_categories boolean NOT NULL,
    show_with_rooms boolean NOT NULL,
    show_with_all_rooms boolean NOT NULL,
    sort_key integer DEFAULT 0 NOT NULL,
    deleted boolean DEFAULT false NOT NULL,
    last_updated timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT announcements_date_range CHECK (((begin_date IS NULL) OR (end_date IS NULL) OR (end_date >= begin_date)))
);


--
-- Name: categories; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.categories (
    id uuid NOT NULL,
    title character varying NOT NULL,
    icon character varying NOT NULL,
    color character(6) NOT NULL,
    event_id integer NOT NULL,
    deleted boolean DEFAULT false NOT NULL,
    last_updated timestamp with time zone DEFAULT now() NOT NULL,
    is_official boolean DEFAULT false NOT NULL,
    sort_key integer DEFAULT 0 NOT NULL
);


--
-- Name: entries; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.entries (
    id uuid NOT NULL,
    title character varying NOT NULL,
    description character varying NOT NULL,
    responsible_person character varying NOT NULL,
    is_room_reservation boolean DEFAULT false NOT NULL,
    event_id integer NOT NULL,
    begin timestamp with time zone NOT NULL,
    "end" timestamp with time zone NOT NULL,
    category uuid NOT NULL,
    deleted boolean DEFAULT false NOT NULL,
    last_updated timestamp with time zone DEFAULT now() NOT NULL,
    comment character varying DEFAULT ''::character varying NOT NULL,
    time_comment character varying DEFAULT ''::character varying NOT NULL,
    room_comment character varying DEFAULT ''::character varying NOT NULL,
    is_exclusive boolean DEFAULT false NOT NULL,
    is_cancelled boolean DEFAULT false NOT NULL,
    CONSTRAINT entries_time_range CHECK (("end" >= begin))
);


--
-- Name: entry_rooms; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.entry_rooms (
    entry_id uuid NOT NULL,
    room_id uuid NOT NULL
);


--
-- Name: event_passphrases; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.event_passphrases (
    id integer NOT NULL,
    event_id integer NOT NULL,
    privilege integer NOT NULL,
    passphrase character varying,
    derivable_from_passphrase integer,
    comment character varying DEFAULT ''::character varying NOT NULL,
    valid_from timestamp with time zone,
    valid_until timestamp with time zone
);


--
-- Name: COLUMN event_passphrases.passphrase; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.event_passphrases.passphrase IS 'if NULL, this passphrase can only derived from another one';


--
-- Name: event_passphrases_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.event_passphrases_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: event_passphrases_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.event_passphrases_id_seq OWNED BY public.event_passphrases.id;


--
-- Name: events; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.events (
    id integer NOT NULL,
    title character varying NOT NULL,
    begin_date date NOT NULL,
    end_date date NOT NULL,
    timezone character varying NOT NULL,
    effective_begin_of_day time without time zone NOT NULL,
    default_time_schedule jsonb NOT NULL,
    slug character varying,
    preceding_event_id integer,
    subsequent_event_id integer,
    CONSTRAINT events_date_range CHECK ((end_date >= begin_date))
);


--
-- Name: events_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.events_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: events_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.events_id_seq OWNED BY public.events.id;


--
-- Name: previous_date_rooms; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.previous_date_rooms (
    previous_date_id uuid NOT NULL,
    room_id uuid NOT NULL
);


--
-- Name: previous_dates; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.previous_dates (
    id uuid NOT NULL,
    entry_id uuid NOT NULL,
    comment character varying NOT NULL,
    begin timestamp with time zone NOT NULL,
    "end" timestamp with time zone NOT NULL,
    last_updated timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT previous_dates_time_range CHECK (("end" >= begin))
);


--
-- Name: rooms; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.rooms (
    id uuid NOT NULL,
    title character varying NOT NULL,
    description character varying NOT NULL,
    event_id integer NOT NULL,
    deleted boolean DEFAULT false NOT NULL,
    last_updated timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: event_passphrases id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.event_passphrases ALTER COLUMN id SET DEFAULT nextval('public.event_passphrases_id_seq'::regclass);


--
-- Name: events id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.events ALTER COLUMN id SET DEFAULT nextval('public.events_id_seq'::regclass);


--
-- Data for Name: __diesel_schema_migrations; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.__diesel_schema_migrations (version, run_on) FROM stdin;
20250531140600	2025-06-15 14:20:26.968825
20250602173009	2025-06-15 14:20:26.986616
20250930072909	2025-09-30 07:49:42.540633
20250930103534	2025-10-03 10:40:26.573708
20251014195940	2025-10-14 20:09:38.243814
20251025113806	2025-10-25 14:32:23.431471
20251108174050	2025-11-08 19:19:48.400034
20251126174535	2025-11-29 16:32:57.663578
\.


--
-- Data for Name: announcement_categories; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.announcement_categories (announcement_id, category_id) FROM stdin;
\.


--
-- Data for Name: announcement_rooms; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.announcement_rooms (announcement_id, room_id) FROM stdin;
\.


--
-- Data for Name: announcements; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.announcements (id, event_id, announcement_type, text, show_with_days, begin_date, end_date, show_with_categories, show_with_all_categories, show_with_rooms, show_with_all_rooms, sort_key, deleted, last_updated) FROM stdin;
\.


--
-- Data for Name: categories; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.categories (id, title, icon, color, event_id, deleted, last_updated, is_official, sort_key) FROM stdin;
\.


--
-- Data for Name: entries; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.entries (id, title, description, responsible_person, is_room_reservation, event_id, begin, "end", category, deleted, last_updated, comment, time_comment, room_comment, is_exclusive, is_cancelled) FROM stdin;
\.


--
-- Data for Name: entry_rooms; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.entry_rooms (entry_id, room_id) FROM stdin;
\.


--
-- Data for Name: event_passphrases; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.event_passphrases (id, event_id, privilege, passphrase, derivable_from_passphrase, comment, valid_from, valid_until) FROM stdin;
\.


--
-- Data for Name: events; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.events (id, title, begin_date, end_date, timezone, effective_begin_of_day, default_time_schedule, slug, preceding_event_id, subsequent_event_id) FROM stdin;
\.


--
-- Data for Name: previous_date_rooms; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.previous_date_rooms (previous_date_id, room_id) FROM stdin;
\.


--
-- Data for Name: previous_dates; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.previous_dates (id, entry_id, comment, begin, "end", last_updated) FROM stdin;
\.


--
-- Data for Name: rooms; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.rooms (id, title, description, event_id, deleted, last_updated) FROM stdin;
\.


--
-- Name: event_passphrases_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('public.event_passphrases_id_seq', 1, false);


--
-- Name: events_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('public.events_id_seq', 1, false);


--
-- Name: __diesel_schema_migrations __diesel_schema_migrations_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.__diesel_schema_migrations
    ADD CONSTRAINT __diesel_schema_migrations_pkey PRIMARY KEY (version);


--
-- Name: announcement_categories announcement_categories_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.announcement_categories
    ADD CONSTRAINT announcement_categories_pkey PRIMARY KEY (announcement_id, category_id);


--
-- Name: announcement_rooms announcement_rooms_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.announcement_rooms
    ADD CONSTRAINT announcement_rooms_pkey PRIMARY KEY (announcement_id, room_id);


--
-- Name: announcements announcements_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.announcements
    ADD CONSTRAINT announcements_pkey PRIMARY KEY (id);


--
-- Name: categories categories_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.categories
    ADD CONSTRAINT categories_pkey PRIMARY KEY (id);


--
-- Name: entries entries_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.entries
    ADD CONSTRAINT entries_pkey PRIMARY KEY (id);


--
-- Name: entry_rooms entry_rooms_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.entry_rooms
    ADD CONSTRAINT entry_rooms_pkey PRIMARY KEY (entry_id, room_id);


--
-- Name: event_passphrases event_passphrases_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.event_passphrases
    ADD CONSTRAINT event_passphrases_pkey PRIMARY KEY (id);


--
-- Name: events events_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.events
    ADD CONSTRAINT events_pkey PRIMARY KEY (id);


--
-- Name: previous_date_rooms previous_date_rooms_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.previous_date_rooms
    ADD CONSTRAINT previous_date_rooms_pkey PRIMARY KEY (previous_date_id, room_id);


--
-- Name: previous_dates previous_dates_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.previous_dates
    ADD CONSTRAINT previous_dates_pkey PRIMARY KEY (id);


--
-- Name: rooms rooms_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.rooms
    ADD CONSTRAINT rooms_pkey PRIMARY KEY (id);


--
-- Name: announcements_event_id_sort_key_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX announcements_event_id_sort_key_idx ON public.announcements USING btree (event_id, sort_key);


--
-- Name: categories_event_id_sort_key_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX categories_event_id_sort_key_idx ON public.categories USING btree (event_id, sort_key);


--
-- Name: entries_event_id_begin_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX entries_event_id_begin_idx ON public.entries USING btree (event_id, begin);


--
-- Name: event_passphrases_event_id_passphrase_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX event_passphrases_event_id_passphrase_idx ON public.event_passphrases USING btree (event_id, passphrase);


--
-- Name: previous_dates_entry_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX previous_dates_entry_id_idx ON public.previous_dates USING btree (entry_id);


--
-- Name: rooms_event_id_title_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX rooms_event_id_title_idx ON public.rooms USING btree (event_id, title);


--
-- Name: announcements sync_lastmod; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER sync_lastmod BEFORE UPDATE ON public.announcements FOR EACH ROW EXECUTE FUNCTION public.sync_lastmod();


--
-- Name: categories sync_lastmod; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER sync_lastmod BEFORE UPDATE ON public.categories FOR EACH ROW EXECUTE FUNCTION public.sync_lastmod();


--
-- Name: entries sync_lastmod; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER sync_lastmod BEFORE UPDATE ON public.entries FOR EACH ROW EXECUTE FUNCTION public.sync_lastmod();


--
-- Name: previous_dates sync_lastmod; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER sync_lastmod BEFORE UPDATE ON public.previous_dates FOR EACH ROW EXECUTE FUNCTION public.sync_lastmod();


--
-- Name: rooms sync_lastmod; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER sync_lastmod BEFORE UPDATE ON public.rooms FOR EACH ROW EXECUTE FUNCTION public.sync_lastmod();


--
-- Name: announcement_categories announcement_categories_announcement_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.announcement_categories
    ADD CONSTRAINT announcement_categories_announcement_id_fkey FOREIGN KEY (announcement_id) REFERENCES public.announcements(id) ON DELETE CASCADE;


--
-- Name: announcement_categories announcement_categories_category_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.announcement_categories
    ADD CONSTRAINT announcement_categories_category_id_fkey FOREIGN KEY (category_id) REFERENCES public.categories(id);


--
-- Name: announcement_rooms announcement_rooms_announcement_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.announcement_rooms
    ADD CONSTRAINT announcement_rooms_announcement_id_fkey FOREIGN KEY (announcement_id) REFERENCES public.announcements(id) ON DELETE CASCADE;


--
-- Name: announcement_rooms announcement_rooms_room_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.announcement_rooms
    ADD CONSTRAINT announcement_rooms_room_id_fkey FOREIGN KEY (room_id) REFERENCES public.rooms(id);


--
-- Name: announcements announcements_event_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.announcements
    ADD CONSTRAINT announcements_event_id_fkey FOREIGN KEY (event_id) REFERENCES public.events(id) ON DELETE CASCADE;


--
-- Name: categories categories_event_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.categories
    ADD CONSTRAINT categories_event_id_fkey FOREIGN KEY (event_id) REFERENCES public.events(id) ON DELETE CASCADE;


--
-- Name: entries entries_category_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.entries
    ADD CONSTRAINT entries_category_fkey FOREIGN KEY (category) REFERENCES public.categories(id);


--
-- Name: entries entries_event_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.entries
    ADD CONSTRAINT entries_event_id_fkey FOREIGN KEY (event_id) REFERENCES public.events(id) ON DELETE CASCADE;


--
-- Name: entry_rooms entry_rooms_entry_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.entry_rooms
    ADD CONSTRAINT entry_rooms_entry_id_fkey FOREIGN KEY (entry_id) REFERENCES public.entries(id) ON DELETE CASCADE;


--
-- Name: entry_rooms entry_rooms_room_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.entry_rooms
    ADD CONSTRAINT entry_rooms_room_id_fkey FOREIGN KEY (room_id) REFERENCES public.rooms(id);


--
-- Name: event_passphrases event_passphrases_derivable_from_passphrase_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.event_passphrases
    ADD CONSTRAINT event_passphrases_derivable_from_passphrase_fkey FOREIGN KEY (derivable_from_passphrase) REFERENCES public.event_passphrases(id);


--
-- Name: event_passphrases event_passphrases_event_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.event_passphrases
    ADD CONSTRAINT event_passphrases_event_id_fkey FOREIGN KEY (event_id) REFERENCES public.events(id) ON DELETE CASCADE;


--
-- Name: events events_preceding_event_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.events
    ADD CONSTRAINT events_preceding_event_id_fkey FOREIGN KEY (preceding_event_id) REFERENCES public.events(id);


--
-- Name: events events_subsequent_event_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.events
    ADD CONSTRAINT events_subsequent_event_id_fkey FOREIGN KEY (subsequent_event_id) REFERENCES public.events(id);


--
-- Name: previous_date_rooms previous_date_rooms_previous_date_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.previous_date_rooms
    ADD CONSTRAINT previous_date_rooms_previous_date_id_fkey FOREIGN KEY (previous_date_id) REFERENCES public.previous_dates(id) ON DELETE CASCADE;


--
-- Name: previous_date_rooms previous_date_rooms_room_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.previous_date_rooms
    ADD CONSTRAINT previous_date_rooms_room_id_fkey FOREIGN KEY (room_id) REFERENCES public.rooms(id);


--
-- Name: previous_dates previous_dates_entry_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.previous_dates
    ADD CONSTRAINT previous_dates_entry_id_fkey FOREIGN KEY (entry_id) REFERENCES public.entries(id) ON DELETE CASCADE;


--
-- Name: rooms rooms_event_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.rooms
    ADD CONSTRAINT rooms_event_id_fkey FOREIGN KEY (event_id) REFERENCES public.events(id) ON DELETE CASCADE;


--
-- Name: SCHEMA public; Type: ACL; Schema: -; Owner: -
--

REVOKE USAGE ON SCHEMA public FROM PUBLIC;
GRANT ALL ON SCHEMA public TO PUBLIC;


--
-- PostgreSQL database dump complete
--

\unrestrict 3wy3LqJmoibxMMyHQiicIQF8XxmAUHcRMGhq9kkNfcYfQgAZoEzwMdM4WN8OLU3

