# Actor Dictionary — Canonical Reference

Canonical list of actor labels used in DRRP extraction. These labels appear in the `governed_actors`, `government_actors`, `duty_holder`, `rights_holder`, `responsibility_holder`, and `power_holder` columns across both fractalaw and sertantai.

**Source of truth**: `fractalaw-core/src/taxa/actors.rs`

When adding actors to fractalaw, update this file and coordinate with sertantai (Baserow schema single-select fields must match).

## Government Actors

Authorities, agencies, ministers, EU institutions. These appear in `government_actors` and `responsibility_holder` / `power_holder` columns.

| Label | Scope |
|-------|-------|
| Crown | UK |
| Gvt: Minister: Secretary of State for Defence | UK |
| Gvt: Minister: Secretary of State for Transport | UK |
| Gvt: Minister: Attorney General | UK |
| Gvt: Minister | UK |
| Gvt: Agency: Health and Safety Executive for Northern Ireland | UK |
| Gvt: Agency: Health and Safety Executive | UK |
| Gvt: Agency: Environment Agency | UK |
| Gvt: Agency: Scottish Environment Protection Agency | UK |
| Gvt: Agency: Office for Nuclear Regulation | UK |
| Gvt: Agency: Office for Environmental Protection | UK |
| Gvt: Agency: Office of Rail and Road | UK |
| Gvt: Agency: OFCOM | UK |
| Gvt: Agency: Natural Resources Body for Wales | UK |
| Gvt: Agency: Maritime and Coastguard Agency | UK |
| Gvt: Agency: Oil and Gas Authority | UK |
| Gvt: Agency | Generic |
| Gvt: Authority: Enforcement | UK |
| Gvt: Authority: Local | UK |
| Gvt: Authority: Planning | UK |
| Gvt: Authority: Fire and Rescue | UK |
| Gvt: Authority: Harbour | UK |
| Gvt: Authority: Licensing | UK |
| Gvt: Authority: Waste | UK |
| Gvt: Authority: Public | UK |
| Gvt: Authority: Traffic | UK |
| Gvt: Authority: Market | UK |
| Gvt: Authority | Generic |
| Gvt: Commissioners | UK |
| Gvt: Officer | UK |
| Gvt: Judiciary | UK |
| Gvt: Emergency Services: Police | UK |
| Gvt: Emergency Services | UK |
| Gvt: Appropriate Person | UK |
| Gvt: Ministry: Treasury | UK |
| Gvt: Ministry: HMRC | UK |
| Gvt: Ministry: Ministry of Defence | UK |
| Gvt: Ministry: Department of Enterprise, Trade and Investment | UK |
| Gvt: Ministry | Generic |
| Gvt: Devolved Admin: National Assembly for Wales | UK |
| Gvt: Devolved Admin: Scottish Parliament | UK |
| Gvt: Devolved Admin: Northern Ireland Assembly | UK |
| Gvt: Devolved Admin | UK |
| HM Forces | UK |
| EU: Commission | EU |
| EU: Member State | EU |
| EU: Agency: ECHA | EU |
| EU: Agency: EFSA | EU |
| EU: Agency: EEA | EU |

## Governed Actors

Businesses, individuals, specialists, supply-chain actors. These appear in `governed_actors` and `duty_holder` columns.

### Core (all law types)

| Label | Category |
|-------|----------|
| Org: Employer | Organisation |
| Org: Owner | Organisation |
| Org: Occupier | Organisation |
| Org: Company | Organisation |
| Operator | Organisation |
| Ind: Employee | Individual |
| Ind: Worker | Individual |
| Ind: Self-employed Worker | Individual |
| Ind: Responsible Person | Individual |
| Ind: Competent Person | Individual |
| Ind: Duty Holder | Individual |
| Ind: Manager | Individual |
| Ind: Supervisor | Individual |
| Ind: Person | Individual |
| Ind: User | Individual |
| Spc: Inspector | Specialist |
| Spc: Employees' Representative | Specialist |
| Spc: Trade Union | Specialist |
| Spc: Assessor | Specialist |
| Spc: Engineer | Specialist |
| SC: Manufacturer | Supply Chain |
| SC: C: Principal Designer | Supply Chain: Construction |
| SC: C: Designer | Supply Chain: Construction |
| SC: C: Principal Contractor | Supply Chain: Construction |
| SC: C: Contractor | Supply Chain: Construction |
| SC: Supplier | Supply Chain |
| SC: Importer | Supply Chain |
| SC: Distributor | Supply Chain |
| SC: Registrant | Supply Chain |
| SC: Downstream User | Supply Chain |
| SC: Applicant | Supply Chain |
| SC: Authorised Representative | Supply Chain |
| SC: Notified Body | Supply Chain |
| SC: Client | Supply Chain |
| SC: T&L: Carrier | Supply Chain: Transport |
| SC: T&L: Driver | Supply Chain: Transport |
| Svc: Installer | Service |
| Org: Landlord | Organisation |
| Public | General |

### Family-gated (only active for matching law families)

| Label | Family gate |
|-------|------------|
| Offshore: Licensee | OH&S: Offshore* |
| Public: Provider | PUBLIC |
| Public: Keeper | PUBLIC |
| Public: Dealer | PUBLIC |
