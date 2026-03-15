// Manually maintained - uses patch semantics where undefined = no change, null = clear, value = set
// Corresponds to Rust UpdateClubRequest with Option<Option<T>> fields

export type UpdateClubRequest = {
	name?: string;
	homeBaseAirportId?: number | null;
	isSoaring?: boolean;
	street1?: string | null;
	street2?: string | null;
	city?: string | null;
	state?: string | null;
	zipCode?: string | null;
	countryCode?: string | null;
};
