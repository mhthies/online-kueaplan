--
-- PostgreSQL database dump
--

-- Dumped from database version 17.4
-- Dumped by pg_dump version 17.4

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
ALTER TABLE ONLY public.event_passphrases DROP CONSTRAINT event_passphrases_event_id_fkey;
ALTER TABLE ONLY public.entry_rooms DROP CONSTRAINT entry_rooms_room_id_fkey;
ALTER TABLE ONLY public.entry_rooms DROP CONSTRAINT entry_rooms_entry_id_fkey;
ALTER TABLE ONLY public.entries DROP CONSTRAINT entries_event_id_fkey;
ALTER TABLE ONLY public.entries DROP CONSTRAINT entries_category_fkey;
ALTER TABLE ONLY public.categories DROP CONSTRAINT categories_event_id_fkey;
DROP TRIGGER sync_lastmod ON public.rooms;
DROP TRIGGER sync_lastmod ON public.previous_dates;
DROP TRIGGER sync_lastmod ON public.entries;
DROP TRIGGER sync_lastmod ON public.categories;
DROP INDEX public.rooms_event_id_idx;
DROP INDEX public.previous_dates_entry_id_idx;
DROP INDEX public.previous_date_rooms_previous_date_id_idx;
DROP INDEX public.event_passphrases_event_id_passphrase_idx;
DROP INDEX public.entry_rooms_entry_id_idx;
DROP INDEX public.entries_event_id_idx;
DROP INDEX public.categories_event_id_idx;
ALTER TABLE ONLY public.rooms DROP CONSTRAINT rooms_pkey;
ALTER TABLE ONLY public.previous_dates DROP CONSTRAINT previous_dates_pkey;
ALTER TABLE ONLY public.previous_date_rooms DROP CONSTRAINT previous_date_rooms_pkey;
ALTER TABLE ONLY public.events DROP CONSTRAINT events_pkey;
ALTER TABLE ONLY public.event_passphrases DROP CONSTRAINT event_passphrases_pkey;
ALTER TABLE ONLY public.entry_rooms DROP CONSTRAINT entry_rooms_pkey;
ALTER TABLE ONLY public.entries DROP CONSTRAINT entries_pkey;
ALTER TABLE ONLY public.categories DROP CONSTRAINT categories_pkey;
ALTER TABLE ONLY public.__diesel_schema_migrations DROP CONSTRAINT __diesel_schema_migrations_pkey;
ALTER TABLE public.rooms ALTER COLUMN event_id DROP DEFAULT;
ALTER TABLE public.events ALTER COLUMN id DROP DEFAULT;
ALTER TABLE public.event_passphrases ALTER COLUMN event_id DROP DEFAULT;
ALTER TABLE public.event_passphrases ALTER COLUMN id DROP DEFAULT;
ALTER TABLE public.entries ALTER COLUMN event_id DROP DEFAULT;
ALTER TABLE public.categories ALTER COLUMN event_id DROP DEFAULT;
DROP SEQUENCE public.rooms_event_id_seq;
DROP TABLE public.rooms;
DROP TABLE public.previous_dates;
DROP TABLE public.previous_date_rooms;
DROP SEQUENCE public.events_id_seq;
DROP TABLE public.events;
DROP SEQUENCE public.event_passphrases_id_seq;
DROP SEQUENCE public.event_passphrases_event_id_seq;
DROP TABLE public.event_passphrases;
DROP TABLE public.entry_rooms;
DROP SEQUENCE public.entries_event_id_seq;
DROP TABLE public.entries;
DROP SEQUENCE public.categories_event_id_seq;
DROP TABLE public.categories;
DROP TABLE public.__diesel_schema_migrations;
DROP FUNCTION public.sync_lastmod();
DROP FUNCTION public.diesel_set_updated_at();
DROP FUNCTION public.diesel_manage_updated_at(_tbl regclass);
-- *not* dropping schema, since initdb creates it
--
-- Name: public; Type: SCHEMA; Schema: -; Owner: -
--

-- *not* creating schema, since initdb creates it


--
-- Name: diesel_manage_updated_at(regclass); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.diesel_manage_updated_at(_tbl regclass) RETURNS void
    LANGUAGE plpgsql
    AS $$
BEGIN
    EXECUTE format('CREATE TRIGGER set_updated_at BEFORE UPDATE ON %s
                    FOR EACH ROW EXECUTE PROCEDURE diesel_set_updated_at()', _tbl);
END;
$$;


--
-- Name: diesel_set_updated_at(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.diesel_set_updated_at() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    IF (
        NEW IS DISTINCT FROM OLD AND
        NEW.updated_at IS NOT DISTINCT FROM OLD.updated_at
    ) THEN
        NEW.updated_at := current_timestamp;
    END IF;
    RETURN NEW;
END;
$$;


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
    is_official boolean DEFAULT false NOT NULL
);


--
-- Name: categories_event_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.categories_event_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: categories_event_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.categories_event_id_seq OWNED BY public.categories.event_id;


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
    is_cancelled boolean DEFAULT false NOT NULL
);


--
-- Name: entries_event_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.entries_event_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: entries_event_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.entries_event_id_seq OWNED BY public.entries.event_id;


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
    passphrase character varying NOT NULL
);


--
-- Name: event_passphrases_event_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.event_passphrases_event_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: event_passphrases_event_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.event_passphrases_event_id_seq OWNED BY public.event_passphrases.event_id;


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
    end_date date NOT NULL
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
    last_updated timestamp with time zone DEFAULT now() NOT NULL
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
-- Name: rooms_event_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.rooms_event_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: rooms_event_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.rooms_event_id_seq OWNED BY public.rooms.event_id;


--
-- Name: categories event_id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.categories ALTER COLUMN event_id SET DEFAULT nextval('public.categories_event_id_seq'::regclass);


--
-- Name: entries event_id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.entries ALTER COLUMN event_id SET DEFAULT nextval('public.entries_event_id_seq'::regclass);


--
-- Name: event_passphrases id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.event_passphrases ALTER COLUMN id SET DEFAULT nextval('public.event_passphrases_id_seq'::regclass);


--
-- Name: event_passphrases event_id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.event_passphrases ALTER COLUMN event_id SET DEFAULT nextval('public.event_passphrases_event_id_seq'::regclass);


--
-- Name: events id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.events ALTER COLUMN id SET DEFAULT nextval('public.events_id_seq'::regclass);


--
-- Name: rooms event_id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.rooms ALTER COLUMN event_id SET DEFAULT nextval('public.rooms_event_id_seq'::regclass);


--
-- Data for Name: __diesel_schema_migrations; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.__diesel_schema_migrations (version, run_on) FROM stdin;
00000000000000	2025-04-10 20:33:02.670301
20220925130147	2025-04-10 20:33:02.672257
20231230160951	2025-04-10 20:33:02.680111
20240602083810	2025-04-10 20:33:02.683794
20250202164157	2025-04-10 20:33:02.686166
20250311195954	2025-04-10 20:33:02.687421
20250315112501	2025-04-10 20:33:02.688083
\.


--
-- Data for Name: categories; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.categories (id, title, icon, color, event_id, deleted, last_updated, is_official) FROM stdin;
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

COPY public.event_passphrases (id, event_id, privilege, passphrase) FROM stdin;
\.


--
-- Data for Name: events; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.events (id, title, begin_date, end_date) FROM stdin;
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
-- Name: categories_event_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('public.categories_event_id_seq', 1, false);


--
-- Name: entries_event_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('public.entries_event_id_seq', 1, false);


--
-- Name: event_passphrases_event_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('public.event_passphrases_event_id_seq', 1, false);


--
-- Name: event_passphrases_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('public.event_passphrases_id_seq', 1, false);


--
-- Name: events_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('public.events_id_seq', 1, false);


--
-- Name: rooms_event_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('public.rooms_event_id_seq', 1, false);


--
-- Name: __diesel_schema_migrations __diesel_schema_migrations_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.__diesel_schema_migrations
    ADD CONSTRAINT __diesel_schema_migrations_pkey PRIMARY KEY (version);


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
-- Name: categories_event_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX categories_event_id_idx ON public.categories USING btree (event_id);


--
-- Name: entries_event_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX entries_event_id_idx ON public.entries USING btree (event_id);


--
-- Name: entry_rooms_entry_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX entry_rooms_entry_id_idx ON public.entry_rooms USING btree (entry_id);


--
-- Name: event_passphrases_event_id_passphrase_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX event_passphrases_event_id_passphrase_idx ON public.event_passphrases USING btree (event_id, passphrase);


--
-- Name: previous_date_rooms_previous_date_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX previous_date_rooms_previous_date_id_idx ON public.previous_date_rooms USING btree (previous_date_id);


--
-- Name: previous_dates_entry_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX previous_dates_entry_id_idx ON public.previous_dates USING btree (entry_id);


--
-- Name: rooms_event_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX rooms_event_id_idx ON public.rooms USING btree (event_id);


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
-- Name: categories categories_event_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.categories
    ADD CONSTRAINT categories_event_id_fkey FOREIGN KEY (event_id) REFERENCES public.events(id);


--
-- Name: entries entries_category_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.entries
    ADD CONSTRAINT entries_category_fkey FOREIGN KEY (category) REFERENCES public.categories(id);


--
-- Name: entries entries_event_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.entries
    ADD CONSTRAINT entries_event_id_fkey FOREIGN KEY (event_id) REFERENCES public.events(id);


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
-- Name: event_passphrases event_passphrases_event_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.event_passphrases
    ADD CONSTRAINT event_passphrases_event_id_fkey FOREIGN KEY (event_id) REFERENCES public.events(id);


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
    ADD CONSTRAINT rooms_event_id_fkey FOREIGN KEY (event_id) REFERENCES public.events(id);


--
-- Name: SCHEMA public; Type: ACL; Schema: -; Owner: -
--

REVOKE USAGE ON SCHEMA public FROM PUBLIC;
GRANT ALL ON SCHEMA public TO PUBLIC;


--
-- PostgreSQL database dump complete
--

