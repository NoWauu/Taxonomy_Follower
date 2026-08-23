-- Locations, trips and the intermediate stops of a trip.
--
-- PostGIS is enabled by the init migration. Positions are stored as
-- GEOGRAPHY(POINT, 4326) rather than GEOMETRY: distances and radius searches on
-- a geography column are answered in metres on the WGS84 spheroid, so no
-- projection step is needed anywhere in the API.
--
-- Note on coordinate order: PostGIS points are (x, y) = (longitude, latitude).
-- Every query in the API therefore builds points with
-- `ST_SetSRID(ST_MakePoint(<lon>, <lat>), 4326)::geography` and reads them back
-- with `ST_X(position::geometry)` for the longitude and `ST_Y(...)` for the
-- latitude.

CREATE TABLE IF NOT EXISTS locations (
    id         SERIAL                 PRIMARY KEY,
    name       VARCHAR(255)           NOT NULL,
    position   GEOGRAPHY(POINT, 4326) NOT NULL,
    created_at TIMESTAMPTZ            NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ            NOT NULL DEFAULT now()
);

-- Required for ST_DWithin / ordering by ST_Distance to use an index scan.
CREATE INDEX IF NOT EXISTS locations_position_idx ON locations USING GIST (position);
CREATE INDEX IF NOT EXISTS locations_name_idx ON locations (lower(name));

CREATE TABLE IF NOT EXISTS trips (
    id                SERIAL      PRIMARY KEY,
    -- Who published the trip. Only this user may edit or cancel it.
    created_by        UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    start_date        TIMESTAMPTZ NOT NULL,
    start_location_id INT         NOT NULL REFERENCES locations (id) ON DELETE RESTRICT,
    end_location_id   INT         NOT NULL REFERENCES locations (id) ON DELETE RESTRICT,
    available_seats   INT         NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Departure and arrival may be the same location: a round trip leaves from
    -- and comes back to the same place, with the route it takes described by
    -- the rows in trip_stops.
    CONSTRAINT trips_available_seats_check CHECK (available_seats >= 0)
);

CREATE INDEX IF NOT EXISTS trips_created_by_idx ON trips (created_by);
CREATE INDEX IF NOT EXISTS trips_start_date_idx ON trips (start_date);
CREATE INDEX IF NOT EXISTS trips_start_location_id_idx ON trips (start_location_id);
CREATE INDEX IF NOT EXISTS trips_end_location_id_idx ON trips (end_location_id);

CREATE TABLE IF NOT EXISTS trip_stops (
    id               SERIAL PRIMARY KEY,
    trip_id          INT    NOT NULL REFERENCES trips (id) ON DELETE CASCADE,
    stop_location_id INT    NOT NULL REFERENCES locations (id) ON DELETE RESTRICT,
    -- Position of the stop along the route, 0-based, start and end excluded.
    stop_order       INT    NOT NULL,
    CONSTRAINT trip_stops_stop_order_check CHECK (stop_order >= 0),
    CONSTRAINT trip_stops_trip_id_stop_order_key UNIQUE (trip_id, stop_order)
);

CREATE INDEX IF NOT EXISTS trip_stops_trip_id_idx ON trip_stops (trip_id);
CREATE INDEX IF NOT EXISTS trip_stops_stop_location_id_idx ON trip_stops (stop_location_id);

-- set_updated_at() is created by the users migration.
DROP TRIGGER IF EXISTS locations_set_updated_at ON locations;
CREATE TRIGGER locations_set_updated_at
    BEFORE UPDATE ON locations
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

DROP TRIGGER IF EXISTS trips_set_updated_at ON trips;
CREATE TRIGGER trips_set_updated_at
    BEFORE UPDATE ON trips
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
